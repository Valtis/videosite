// Utility functions for using the ErrorBanner component across different pages

/**
 * Shows an error message using the ErrorBanner component
 * @param {string} message - The error message to display
 * @param {HTMLElement} container - Optional container element. Defaults to document.body
 * @returns {ErrorBanner} The error banner element
 */
function showErrorMessage(message, container = document.body) {
    return ErrorBanner.showError(message, container);
}

/**
 * Shows an error message specifically for form validation
 * @param {string} message - The validation error message
 * @param {HTMLElement} formContainer - The form container element
 * @returns {ErrorBanner} The error banner element
 */
function showFormError(message, formContainer) {
    return ErrorBanner.showError(message, formContainer);
}

/**
 * Shows an error message for API/network errors
 * @param {Error} error - The error object
 * @param {HTMLElement} container - Optional container element
 * @returns {ErrorBanner} The error banner element
 */
function showAPIError(error, container = document.body) {
    const message = error.message || "An unexpected error occurred. Please try again.";
    return ErrorBanner.showError(message, container);
}

/**
 * Hides all error banners in a container
 * @param {HTMLElement} container - The container to search for error banners
 */
function hideAllErrors(container = document.body) {
    const errorBanners = container.querySelectorAll('error-banner');
    errorBanners.forEach(banner => banner.hide());
}

// Export for module use if needed
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        showErrorMessage,
        showFormError,
        showAPIError,
        hideAllErrors
    };
}
