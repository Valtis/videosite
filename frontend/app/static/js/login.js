"use strict";


function checkLoginStatus() {
    fetch('/auth/status', {
        credentials: 'include' // Include cookies for authentication
    }).then(response => {
        if (response.ok) {
            // user is already logged in, redirect to index page
            window.location.href = "index.html";
        }
    }).catch(error => {
        console.error('Error checking login status:', error);
    });
}

async function login() {
    const username = document.getElementById("username").value;
    const password = document.getElementById("password").value;

    if (!username || !password) {
        ErrorBanner.showError("Please enter both username and password.", document.getElementById('login_container'));
        return;
    }

    // login request - this will set a
    try {
        let result = await fetch('/auth/login', {
            credentials: 'include', // JWT stored in cookies
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ username, password })
        })

        let json = await result.json();

        if (json.msg) {
            window.location.href = "index.html"; // Redirect to index page on success
        }

        ErrorBanner.showError("Login failed: " + json.err, document.getElementById('login_container'));
    } catch (error) {
        console.error('Error during login:', error);
        ErrorBanner.showError("An error occurred during login. Please try again later.", document.getElementById('login_container'));
    }
}