class ErrorBanner extends HTMLElement {
    constructor() {
        super();
        this.attachShadow({ mode: 'open' });
        this.render();
    }

    render() {
        this.shadowRoot.innerHTML = `
            <style>
                :host {
                    display: none;
                    width: 100%;
                    box-sizing: border-box;
                }

                :host([visible]) {
                    display: block;
                    animation: slideDown 0.3s ease-out;
                }

                .error-banner {
                    background-color: #f8d7da;
                    color: #721c24;
                    border: 1px solid #f5c6cb;
                    border-radius: 4px;
                    padding: 1rem;
                    margin-bottom: 1rem;
                    position: relative;
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                }

                .error-content {
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                }

                .error-message {
                    flex: 1;
                    font-weight: 500;
                }

                .error-icon {
                    margin-right: 0.5rem;
                    font-weight: bold;
                    color: #721c24;
                }

                .close-button {
                    background: none;
                    border: none;
                    font-size: 1.2rem;
                    cursor: pointer;
                    color: #721c24;
                    padding: 0;
                    margin-left: 1rem;
                    line-height: 1;
                }

                .close-button:hover {
                    opacity: 0.7;
                }

                @keyframes slideDown {
                    from {
                        opacity: 0;
                        transform: translateY(-10px);
                    }
                    to {
                        opacity: 1;
                        transform: translateY(0);
                    }
                }
            </style>
            <div class="error-banner">
                <div class="error-content">
                    <span class="error-icon">⚠️</span>
                    <span class="error-message"></span>
                    <button class="close-button" aria-label="Close error message">&times;</button>
                </div>
            </div>
        `;

        this.shadowRoot.querySelector('.close-button').addEventListener('click', () => {
            this.hide();
        });
    }

    show(message) {
        this.shadowRoot.querySelector('.error-message').textContent = message;
        this.setAttribute('visible', '');
        
        // Auto-hide after 5 seconds
        setTimeout(() => {
            this.hide();
        }, 5000);
    }

    hide() {
        this.removeAttribute('visible');
    }

    // Static method to create and show an error banner
    static showError(message, container = document.body) {
        let existingBanner = container.querySelector('error-banner');
        
        if (!existingBanner) {
            existingBanner = document.createElement('error-banner');
            container.insertBefore(existingBanner, container.firstChild);
        }
        
        existingBanner.show(message);
        return existingBanner;
    }
}

// Define the custom element
customElements.define('error-banner', ErrorBanner);

// Export for module use if needed
if (typeof module !== 'undefined' && module.exports) {
    module.exports = ErrorBanner;
}
