"use strict";

async function initPlayer() {

    // Install built-in polyfills to patch browser incompatibilities.
    shaka.polyfill.installAll();

    // Check if the browser supports the necessary APIs.
    if (!shaka.Player.isBrowserSupported()) {
        console.error('Browser not supported!');
        return;
    }

    // Get the video element
    const video = document.getElementById('video');
    const videoContainer = document.getElementById('videoContainer');

    // Create a Player instance first
    const player = new shaka.Player();
    player.attach(video);
    // Listen for error events.
    // Load an HLS manifest

    // get resource_id from query parameters
    const urlParams = new URLSearchParams(window.location.search);
    const resourceId = urlParams.get('resource_id');
    if (!resourceId) {
        console.error("Resource ID not provided in the URL.");
        return;
    }

    const hlsUrl = `/resource/${resourceId}/master.m3u8`; // Example HLS URL



    // Create the UI overlay with the player instance
    const ui = new shaka.ui.Overlay(player, videoContainer, video);

    const config = {
        'controlPanelElements': [
        'play_pause',
        'mute',
        'volume',
        'time_and_duration',
        'spacer',
        'fullscreen',
        'overflow_menu',
        ],
        'overflowMenuButtons': [
        'airplay',
        'cast',
        'quality',
        ],
        'addSeekBar': true
    };
    await ui.configure(config);
    
    await player.load(hlsUrl).then(function () {
        console.log('The video has now been loaded!');
    }).catch(function(error) {
        onError(error);
    });


    // Attach player to the window to make it easy to access in the JS console.
    window.player = player;
}

    // Add a simple error handler for Shaka Player
function onError(error) {
    console.error('Error code', error.code, 'object', error);
}