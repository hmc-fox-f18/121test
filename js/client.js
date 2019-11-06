/*jshint esversion: 6 */
//TODO: Consider using JS modules for imports

//TODO: Consider splitting game logic into separate code from base
// client?

// *******  Constants   ******
const bgColor= "black";
const strokeStyle = "white";

// ******   Variables   ********
//TODO: Consider removing global variables or moving relevant parts to
//other components
var boardWidth;
var boardHeight;

var canvasWidth;
var canvasHeight;

var blockHeight;
var blockWidth;

var board;
var ctx;

var my_player_id;
var keystate = [];
var socket;
var socketOpen = false;

var game_state = new GameState([], [], []);
// Actual Code

/**
 *  Initializes the client state and all game logic variables
 */
function init() {
    //TODO: Receive board specifications from server
    boardWidth = 20;
    boardHeight = 20;
    board = Array(boardWidth).fill(Array(boardHeight));

    canvas = document.getElementById('board');
    ctx = canvas.getContext('2d');
    ctx.strokeStyle = strokeStyle;

    canvasWidth = canvas.width;
    canvasHeight = canvas.height;

    blockHeight = canvasHeight / boardHeight;
    blockWidth = canvasWidth / boardWidth;

    // store keypresses
    window.addEventListener('keydown', (e) => {
        keystate[e.key] = true;
        e.preventDefault();
    });


    //Initialize network and rendering components
    //TODO: delegate to relevant components
    initSocket(() => {
      initGrid();
      clearBoard();
      drawPieces();
      window.requestAnimationFrame(handleFrame);
    });
}

/**
 *  Delegates the logic for each frame and makes calls to the network
 *  and rendering logic to draw the frame and send data to the server
 */
function handleFrame() {
    updatePosition();
    keystate = [];
    draw_frame();
    window.requestAnimationFrame(handleFrame);
}

/**
 *  Uses the input buffer for the current frame to adjust player
 *  position and rotation.
 */
function updatePosition() {
    var myPiece = getMyPiece();
    // if we do not have a piece yet, do not take input
    if (!myPiece) {
      return;
    }

    // send update piece position to the server
    //const piece_obj = JSON.stringify(myUpdatedPiece);
    sendInput(keystate);
}
