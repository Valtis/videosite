"use strict";


// For player.html, can be ignored if initPlayer is used elsewhere
function onLoadPlayer() {
    const resourceId = new URLSearchParams(window.location.search).get('resource_id');
    if (!resourceId) {
        console.error("Resource ID not provided in the URL.");
        return;
    }

    const video = document.getElementById('video');
    // set the poster attribute to the thumbnail url
    video.poster = `/resource/${resourceId}/thumbnail.jpg`;

    const videoContainer = document.getElementById('videoContainer');
    
    Promise.all([
        initPlayer(videoContainer, video, resourceId),
        createOpenGraphHeaderElements(resourceId),
    ]).catch(error => {
        console.error('Error initializing player or Open Graph tags:', error);
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

async function createOpenGraphHeaderElements(resourceId) {
    // create opengraph meta tags for the resource
    // we use: og:title, og:type, og:image, og:url, og:description
    // for video, we additionally use: og:video, og:video:type, og:video:width, og:video:height


    // {"id":"13be7cd1-ac1d-45c1-a79a-ec604a58f7d8","name":"20250824_135421000_iOS.MOV","status":"processed","type":"video","width":1920,"height":1080,"duration_seconds":1082130432,"bit_rate":12582912,"frame_rate":59.0}
    let metadata = await fetch(`/resource/${resourceId}/metadata`);

    if (!metadata.ok) {
        console.error("Failed to fetch resource metadata:", metadata.statusText);
        return;
    }
    let json = await metadata.json();

    // we do not support descriptions yet
    let description = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.";

    let metaTags = [
        { property: "og:title", content: json.name },
        { property: "og:type", content: "video.other" },
        { property: "og:image", content: `${window.location.origin}/resource/${resourceId}/thumbnail.jpg` },
        { property: "og:url", content: window.location.href },
        { property: "og:description", content: description },
    ];

    if (json.type === "video") {

        let videoTags = [ 
            { property: "og:video", content: `${window.location.origin}/player.html?resource_id=${resourceId}` },
            { property: "og:video:type", content: "text/html" },
            { property: "og:video:width", content: json.width },
            { property: "og:video:height", content: json.height },
        ];

        metaTags = metaTags.concat(videoTags);
    }
    metaTags.forEach(tagInfo => {
        let metaTag = document.createElement('meta');
        metaTag.setAttribute('property', tagInfo.property);
        metaTag.setAttribute('content', tagInfo.content);
        document.head.appendChild(metaTag);
    });

}