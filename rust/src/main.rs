extern crate ws;
extern crate rand;
extern crate slab;

mod piece_state;
mod input;
mod tetris;
mod tests;

use crate::piece_state::{PieceState, Pivot};
use crate::input::{KeyState};
use crate::tetris::update_state;

use rand::Rng;
use std::sync::{Arc, Mutex};
use std::{time, thread};

use ws::{CloseCode, Handler, Handshake, Message, Result,
     Sender, WebSocket, util::Token, util::Timeout};

use slab::Slab;
use serde_json::json;

const FRAME_MILLIS : u64 = (1000.0 / 60.0) as u64;
const FRAME_TIME : time::Duration = time::Duration::from_millis(FRAME_MILLIS);

/**
 *
 * The representation of an individual client
 *
 * TODO: Implement saving data frames for rollback?
 *
 * TODO: Split client into separate module for code clarity?
 */
struct Client<'a> {
    out: Sender,
    player_key: usize,
    players: &'a Mutex<Slab<PieceState>>
}

impl Handler for Client<'_> {
    /**
     *
     * Function called when a connection is opened with a client
     *
     * Clients are added to the shared players Slab, and the initial
     * state is messaged back to the client.
     *
     * TODO: Consider breaking new vs. returning client to different
     * helper methods
     *
     */
    fn on_open(&mut self, shake: Handshake) -> Result<()> {
        println!("Request: {}", shake.request);
        let player_id : usize = self.out.token().into();
        let mut players = self.players.lock().unwrap();
        let response;

        println!("Players: {:?}", players);
        // Resend data for reconnecting user
        // TODO: Resend positional and rotational data ?
        // Could wait on game state update for data instead
        if players.contains(player_id) {
            let new_piece_state = players.get(player_id).unwrap();
            let piece_type = new_piece_state.shape;
            response = json!({
                "player_id": player_id,
                "piece_type": piece_type,
                "type": "init"
            });
        }
        else {
            // Player doesn't exist, add to players list
            // TODO: Genericize initial piece state
            let piece_type: u8 = next_piece();
            let new_piece_state = PieceState{
                shape: piece_type,
                pivot: Pivot{
                    x: 5,
                    y: 5
                },
                rotation: 0,
                player_id: player_id
            };
            // Insert new player data into game state
            self.player_key = players.insert(new_piece_state);
            response = json!({
                "player_id": player_id,
                "piece_type": piece_type,
                "type": "init"
            });
        }
        self.out.send(response.to_string())
    }

    //TODO: Deal with different messages if applicable
    fn on_message(&mut self, msg: Message) -> Result<()> {
        match self.out.timeout(3_000, self.out.token()) {
                    _ => (),
        };
        // Parse the msg as text
        if let Ok(text) = msg.into_text() {
            // Try to parse the message as a piece state
            match serde_json::from_str::<KeyState>(&text) {
                Ok(mut player_input) => {
                    let mut players = self.players.lock().unwrap();
                    // Don't trust input, ensure labelled properly
                    let player_id : usize = self.out.token().into();
                    player_input.player_id = player_id;
                    // Update state for player
                    update_state(&mut players, &player_input);
                    return Ok(());
                }
                Err(e) => {
                    // Piece state is not valid
                    println!("Could not parse status: {}\n", e);
                    return Ok(());
                },
            }
        }
        // default to blank result if message is not parseable
        return Ok(());
    }

    /**
     *
     * Method invoked when a client ceases to be connected
     * to the server.
     *
     * Sets a timeout to remove a client
     *
     * TODO: Add more complex behavior for a more seamless tetris game
     *
     */
    fn on_close(&mut self, code: CloseCode, reason: &str) {
        // Print reason for connection loss
        let player_id : usize = self.out.token().into();
        match code {
            CloseCode::Normal => {
                println!("Client {} is done with the connection.", player_id);
                // TODO: Consider error handling if appropriate
                match self.out.timeout(3_000, self.out.token()) {
                    _ => (),
                };
            }
            CloseCode::Away => {
                println!("Client {} is leaving the site.", player_id);
                // TODO: Consider error handling if appropriate
                match self.out.timeout(3_000, self.out.token()) {
                    _ => (),
                };
            }
            _ => {
                println!("Client {} encountered an error: {}", player_id, reason);
                remove_player(self.out.token(), self.players);
            }
        }
    }

    /**
     *
     *  Method invoked when a client times out.
     *
     *  Logs the disconnection, then proceeds to remove the player
     *  from the game state.
     *
     */
    fn on_timeout(&mut self, event: Token) -> Result<()> {
        // Remove client from game state
        let player_id : usize = self.out.token().into();
        println!("Client {} timed out.", player_id);
        remove_player(event, self.players);
        Ok(())
    }

    /**
     *
     *  Code called when a new timeout event is created.
     *
     *  Should be usable to cancel previous timeouts as data is
     *  received from the client
     *
     *  //TODO: Make this actually work properly
     *
     */
    fn on_new_timeout(&mut self, _event: Token, timeout: Timeout) -> Result<()> {
        self.out.cancel(timeout)
    }
}

/**
 *
 *  Function which removes a given player from the player slab.
 *
 */
fn remove_player(_player_key: Token,
                    _players: &Mutex<Slab<PieceState>>) {
    // Remove client from game state
    //let player_id : usize = player_key.into();
    //let mut players = players.lock().unwrap();
    //players.remove(player_id);
    //drop(players);
}

/**
 *
 *  Generates the next piece to be output
 *
 *  TODO: Implement Tetris bag generation for better distribution
 *
 */
pub fn next_piece() -> u8 {
    let mut rng = rand::thread_rng();
    return rng.gen_range(0, 7);
}

/**
 *
 *  Runs the actual game logic at regular intervals, then sends out a
 *  state update to all the clients.
 *
 */
fn game_frame(broadcaster: Sender,
                thread_players: Arc<Mutex<Slab<PieceState>>>) {
    loop {
        let players = thread_players.lock().unwrap();

        // Parse actual player states out of the list to exclude
        // empty slots in Slab
        let states : Vec<&PieceState> = players
                            .iter()
                            .map(|(_key, val)| val)
                            .collect();

        let response = json!({
            "piece_states": states,
            "type": "gameState"
        });
        //println!("{:?}", states);
        // Unlock players so main thread can take in player updates
        drop(players);
        // Send game state update to all connected clients
        match broadcaster.send(response.to_string()) {
            Ok(v) => v,
            Err(e) => println!("Unable to broadcast info: {}", e)
        };

        // Wait until next frame
        thread::sleep(FRAME_TIME);
    }
}


/**
 *
 *  The code which initializes the server.
 *
 *  After this block is executed, the main thread will take care
 *  of the incoming client updates, while the _game_thread will run
 *  the server logic and send out game state updates
 *
 *
 */
fn main() {
    let players = Arc::new(Mutex::new(Slab::new()));
    let thread_players = players.clone();
    // Code that initializes client structs
    let server_gen  = |out : Sender| {
        Client {
            out: out,
            player_key: 0,
            players: &players
        }
    };

    // Same functionality as listen command, but actually compiles?
    let socket = WebSocket::new(server_gen).unwrap();
    let socket = match socket.bind("127.0.0.1:3012") {
        Ok(v) => v,
        Err(_e) => {
            panic!("Socket in Use, Please Close Other Server")
        },
    };

    // Clone broadcaster to send data to clients on other thread
    let broadcaster = socket.broadcaster().clone();
    let _game_thread = thread::spawn(move || {
        game_frame(broadcaster, thread_players);
    });
    // Run the server on this thread
    socket.run().unwrap();
}
