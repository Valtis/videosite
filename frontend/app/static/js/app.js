"use strict";


function onLoad() {
    load_resources();
    initQuota();
    checkBanners();
    setUserName();
    setInterval(() => {
        load_resources();
    }, 5000);
}

async function load_resources() {
    // cookie for auth
    let response = await fetch(
        "/resource/list", {
            credentials: "include"
        }
    );

    if (!response.ok) {
        console.error("Failed to load resources:", response.statusText);

        if (response.status === 401) {
            // either not logged in or session expired
            window.location.href = "login.html";
        }

        return;
    }

    let resources = await response.json();
    let resourcesList = document.getElementById("resource-container");

    // create a grid for resources. For now, we just show the stats. TODO replace with a real grid
    resources.forEach(resource => { 
        // check for presence of resource div with the same id
        // If it does not exist, create a new one.
        // if the resource div already exists, check the status and type fields. If identical, skip.
        // otherwise, delete and recreate.

        const existingResourceDiv = document.getElementById(`resource-${resource.id}`);
        if (existingResourceDiv) {
            if (existingResourceDiv.status === resource.resource_status && existingResourceDiv.resource_type === resource.resource_type) {
                console.log(`Resource ${resource.id} already exists with the same status and type. Skipping update.`);
                return; // skip if the status and type are the same
            } else {
                console.log(`Resource ${resource.id} exists but with different status or type. Updating...`);
                resourcesList.removeChild(existingResourceDiv); // remove the old div
            }
        } else {
            console.log(`Creating new resource div for ${resource.id}.`);
        }
        
        const resourceDiv = document.createElement("div");
        resourceDiv.className = "resource-item";
        resourceDiv.id = `resource-${resource.id}`;
        resourceDiv.status = resource.resource_status;
        resourceDiv.resource_type = resource.resource_type;
        resourceDiv.created_at = resource.created_at;

        if (resource.resource_type === "video" && resource.resource_status === "processed") {
            const playerDiv = createVideoPlayerElement(resource);
            resourceDiv.appendChild(playerDiv);

            // link to player page
            const playerLink = document.createElement("a");
            playerLink.href = `player.html?resource_id=${resource.id}`;
            playerLink.textContent = "Video Page";
            resourceDiv.appendChild(playerLink);
        } else {
            const generic_div = createGenericElement(resource);
            resourceDiv.appendChild(generic_div);
        }



        const publicP = document.createElement("p");
        const isPublicCheckbox = document.createElement("input");
        isPublicCheckbox.type = "checkbox";
        isPublicCheckbox.checked = resource.is_public;
        publicP.textContent = "Public: ";
        publicP.appendChild(isPublicCheckbox);
        isPublicCheckbox.addEventListener("change", () => {
            update_public_status(resource.id, isPublicCheckbox.checked);
        });

        resourceDiv.appendChild(publicP);

        // Append to the resources list, but determine the proper position
        // based on the created_at timestamp - order by created_at descending order
        if (resourcesList.children.length === 0) {
            resourcesList.appendChild(resourceDiv);
        } else {
            let inserted = false;
            for (let i = 0; i < resourcesList.children.length; i++) {
                const child = resourcesList.children[i];
                if (new Date(child.created_at) < new Date(resource.created_at)) {
                    resourcesList.insertBefore(resourceDiv, child);
                    inserted = true;
                    break;
                }
            }
            if (!inserted) {
                resourcesList.appendChild(resourceDiv);
            }
        }
    });
}

function createVideoPlayerElement(resource) {
    const containerDiv = document.createElement("div");
    containerDiv.className = "video-player-container";
    containerDiv.id = "videoContainer-" + resource.id;

    const videoElement = document.createElement("video");
    videoElement.id = "video-" + resource.id;

    videoElement.className = "video-element";

    containerDiv.appendChild(videoElement);
    initPlayer(containerDiv, videoElement, resource.id); 

    return containerDiv;
}


function createGenericElement(resource) { 

    const containerDiv = document.createElement("div");
    
    // Create elements for resource details
    const h2 = document.createElement("h2");
    h2.textContent = resource.resource_name;

    const typeP = document.createElement("p");
    typeP.textContent = `Type: ${resource.resource_type}`;

    const statusP = document.createElement("p");
    statusP.textContent = `Status: ${resource.resource_status}`;

    const createdP = document.createElement("p");
    createdP.textContent = `Created at: ${new Date(resource.created_at).toLocaleString()}`;


    // Append elements to resourceDiv
    containerDiv.appendChild(h2);
    containerDiv.appendChild(typeP);
    containerDiv.appendChild(statusP);
    containerDiv.appendChild(createdP);

    return containerDiv;
}


async function update_public_status(id, is_public) {
    try {
        let response = await fetch(`/resource/${id}/public`, {
            method: 'POST',
            credentials: 'include',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ is_public })
        });

        if (!response.ok) {
            throw new Error(`Failed to update public status: ${response.statusText}`);
        }

        let result = await response.json();
        console.log("Public status updated:", result);
    } catch (error) {
        console.error('Error updating public status:', error);
    }
}


async function initQuota() {
    try {
        let response = await fetch('/upload/quota', {
            credentials: 'include'
        });

        if (!response.ok) {
            throw new Error(`Failed to fetch quota: ${response.statusText}`);
        }

        let quotaData = await response.json();

        let used_quota = quotaData.used_quota || 0;
        let total_quota = quotaData.total_quota || 0;
        let used_quota_index = 0;
        let total_quota_index = 0;

        const units = ['B', 'KB', 'MB', 'GB', 'TB'];


        while (used_quota > 1024) {
            used_quota /= 1024;
            used_quota_index++;
        }

        while (total_quota > 1024) {
            total_quota /= 1024;
            total_quota_index++;
        }

        let tooltip = `${used_quota.toFixed(2)} ${units[used_quota_index]} / ${total_quota.toFixed(2)} ${units[total_quota_index]}`;
        document.getElementById('quota-tooltip').innerText = tooltip;

        let used_quota_ratio = Math.round(total_quota > 0 ? (quotaData.used_quota / quotaData.total_quota) * 100 : 0);

        const quota_circle_radius_pixels = 40;

        const quota_svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
        quota_svg.setAttribute("width", quota_circle_radius_pixels);
        quota_svg.setAttribute("height", quota_circle_radius_pixels);
        quota_svg.setAttribute("viewBox", "0 0 40 40");
        quota_svg.setAttribute("style", `--progress: ${used_quota_ratio}`);

        const bg_circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
        bg_circle.setAttribute("class", "bg");

        const fg_circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
        fg_circle.setAttribute("class", "fg");
        
        quota_svg.appendChild(bg_circle);
        quota_svg.appendChild(fg_circle);

        quota_svg.classList.add("quota");
        
        // decides the color
        if (used_quota_ratio < 50) {
            quota_svg.classList.add("low");
        } else if (used_quota_ratio < 80) {
            quota_svg.classList.add("medium");
        } else {
            quota_svg.classList.add("high");
        }

        document.getElementById('quota').innerHTML = '';
        document.getElementById('quota').appendChild(quota_svg);

    } catch (error) {
        console.error('Error fetching quota:', error);
    }
}


async function checkBanners() {
    // check error query parameter
    const urlParams = new URLSearchParams(window.location.search);
    const error = urlParams.get('error');

    if (error === 'quota_exceeded') {
        document.getElementById('quota-exceeded-banner').classList.remove('hidden');
    } else {
        document.getElementById('quota-exceeded-banner').classList.add('hidden');
    }
}


function setUserName() {
    // Fetch the username from the server
    fetch('/auth/info', {
        credentials: 'include'
    })
    .then(response => {
        if (!response.ok) {
            throw new Error('Failed to fetch user info');
        }
        return response.json();
    })
    .then(data => {
        const usernameDisplay = document.getElementById('username-display');
        usernameDisplay.textContent = data.display_name;
    })
    .catch(error => {
        console.error('Error fetching user info:', error);
    });
}