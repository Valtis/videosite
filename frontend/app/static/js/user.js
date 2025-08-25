"use strict";

function onLoadUser() {
    // check query params, see if error is present
    const params = new URLSearchParams(window.location.search);
    const error = params.get('error');

    if (error == 'password_mismatch') {
        ErrorBanner.showError("New password and confirmation do not match.", document.getElementById('change_password'));
    } else if (error == 'invalid_current_password') {
        ErrorBanner.showError("Current password is incorrect.", document.getElementById('change_password'));
    } else if (error == 'weak_password') {
        ErrorBanner.showError("New password is too weak. Please choose a stronger password.", document.getElementById('change_password'));
    } else if (error == 'unauthorized') {
        ErrorBanner.showError("You must be logged in to change your password.", document.getElementById('change_password'));
    } else if (error == 'same_password') {
        ErrorBanner.showError("New password must be different from the current password.", document.getElementById('change_password'));
    } else if (error == 'empty_fields') {
        ErrorBanner.showError("All password fields are required.", document.getElementById('change_password'));
    } else if (error) {
        ErrorBanner.showError("An unknown error occurred. Please try again.", document.getElementById('change_password'));
    }

    // remove the query parameter so that refreshing the page doesn't show the error again
    if (error) {
        const url = new URL(window.location);
        url.searchParams.delete('error');
        window.history.replaceState({}, document.title, url.toString());
    }
}


// check if old password is present, new password and its confirmation matches, 
// and if new password is strong enough
// if so, enable the submit button
//
// If all fields contains values, and we have a problem, display an error banner
function checkPasswordForm(showError) {
    const currentPassword = document.getElementById('current_password').value;
    const newPassword = document.getElementById('new_password').value;
    const confirmPassword = document.getElementById('confirm_password').value;
    
    const submitButton = document.getElementById('change_password_submit');
    const changePasswordDiv = document.getElementById('change_password');
    
    // all missing so no nagging under any case
    if (!currentPassword.trim() || !newPassword.trim() || !confirmPassword.trim()) {
        submitButton.disabled = true;
        return;
    }

    if (!currentPassword.trim()) {
        if (showError) {
            ErrorBanner.showError("Current password is required.", changePasswordDiv);
        }
        submitButton.disabled = true;
        return;
    }

    if (newPassword !== confirmPassword) {
        if (showError) {
            ErrorBanner.showError("New password and confirmation do not match.", changePasswordDiv);
        }
        submitButton.disabled = true;
        return;
    }

    const entropy = estimatePasswordStrength(newPassword);
    if (entropy < 70) { 
        if (showError) {
            ErrorBanner.showError("New password is too weak. Please choose a stronger password.", changePasswordDiv);
        }
        submitButton.disabled = true;
        return;
    }

    if (currentPassword === newPassword) {
        if (showError) {
            ErrorBanner.showError("New password must be different from the current password.", changePasswordDiv);
        }
    }

    // if we reach here, everything is good
    submitButton.disabled = false;

}


function estimatePasswordStrength(password) {
    // see if any of the following categories are present:
    // lowercase, uppercase, digits, special characters
    // if present, add the size of the set to the total set
    // use this to create estimate of per-character entropy,
    // and then multiply by length to get total entropy
    //
    // We do not grant any bonus-entropy for any other
    // for the per-character entropy calculation, so
    // this function generally underestimates the strength
    // 
    // and yes, this is very latin-alphabet centric, but
    // that's acceptable for now at least, given the expected
    // user base.

    if (!password || password.trim().length === 0) {
        return 0;
    }

    let charsets = [
        'abcdefghijklmnopqrstuvwxyz',
        'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
        '0123456789',
        '!@#$%^&*()-_=+[]{}|;:\'",.<>?/`~'
    ];

    let charset_size = 0;
    // we just use the index of charsets to track which sets have been included
    let included_sets = [];
    for (let char of password) {
        for (let i = 0; i < charsets.length; i++) {
            if (charsets[i].includes(char) && !included_sets.includes(i)) {
                included_sets.push(i);
                charset_size += charsets[i].length;
            }
        }
    }

    // bits of entropy per character is log2(charset_size)
    let bits_per_char = Math.log2(charset_size);

    return bits_per_char * password.length;
}

