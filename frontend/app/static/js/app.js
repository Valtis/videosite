"use strict";


function loadResourcesPoll() {
    // load immediatly and poll for 5
    load_resources();
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
        return;
    }

    let resources = await response.json();
    let resourcesList = document.getElementById("resources-container");

    // clear the resources container
    resourcesList.innerHTML = "";
    // create a grid for resources. For now, we just show the stats. TODO replace with a real grid
    resources.forEach(resource => { 
        let resourceDiv = document.createElement("div");
        resourceDiv.className = "resource-item";
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
        resourceDiv.appendChild(h2);
        resourceDiv.appendChild(typeP);
        resourceDiv.appendChild(statusP);
        resourceDiv.appendChild(createdP);
        resourcesList.appendChild(resourceDiv);
    });



    // create a grid for resources ()
}