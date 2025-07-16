"use strict";


// For player.html, can be ignored if initPlayer is used elsewhere
function onLoadPlayer() {
    const resourceId = new URLSearchParams(window.location.search).get('resource_id');
    if (!resourceId) {
        console.error("Resource ID not provided in the URL.");
        return;
    }

    const video = document.getElementById('video');
    const videoContainer = document.getElementById('videoContainer');
    
    initPlayer(videoContainer, video, resourceId).then(() => {
        console.log('Player initialized successfully.');
    }).catch(error => {
        console.error('Error initializing player:', error);
    });
}

async function initPlayer(videoContainer, videoElement, resourceId) {

    shaka.polyfill.installAll();

    if (!shaka.Player.isBrowserSupported()) {
        console.error('Browser not supported!');
        return;
    }

    if (!videoElement || !(videoElement instanceof HTMLVideoElement)) {
        console.error(`Element with ID ${videoElement.id} is not a valid video element.`);
        return;
    }

    const player = new shaka.Player();
    player.attach(videoElement);

    const hlsUrl = `/resource/${resourceId}/master.m3u8`;

    const ui = new shaka.ui.Overlay(player, videoContainer, videoElement);

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
        'playback_rate',
        ],
        'addSeekBar': true
    };
    await ui.configure(config);
    
    await player.load(hlsUrl).then(function () {
        console.log('The video has now been loaded!');
    }).catch(function(error) {
        onError(error);
    });


    window.player = player;
}

function onError(error) {
    console.error('Error code', error.code, 'object', error);
}