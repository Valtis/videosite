# Error Banner Component Usage

The Error Banner is a reusable Web Component that displays error messages in a consistent, user-friendly way across your application.

## Features

- **Web Component**: Works in any modern browser without framework dependencies
- **Auto-dismiss**: Automatically hides after 5 seconds
- **Manual close**: Users can close the banner by clicking the × button
- **Accessible**: Includes proper ARIA labels and semantic HTML
- **Animated**: Smooth slide-down animation when displayed
- **Styled**: Consistent error styling with Bootstrap-like colors

## Basic Usage

### 1. Include the Component

Add the error banner component script to your HTML:

```html
<script src="/static/js/components/error-banner.js"></script>
```

### 2. Add the Element to Your HTML

```html
<error-banner></error-banner>
```

### 3. Show Error Messages

```javascript
// Method 1: Using the static showError method
ErrorBanner.showError("Your error message here");

// Method 2: Using the element directly
const banner = document.querySelector('error-banner');
banner.show("Your error message here");

// Method 3: Create and show in one call
ErrorBanner.showError("Error message", document.getElementById('container'));
```

## Usage Examples

### Login Form (Current Implementation)

```html
<div id="login_container">
    <error-banner></error-banner>
    <h2>Login</h2>
    <!-- form content -->
</div>
```

```javascript
// Show validation error
if (!username || !password) {
    ErrorBanner.showError("Please enter both username and password.", 
                         document.getElementById('login_container'));
}

// Show API error
ErrorBanner.showError("Login failed: " + json.err, 
                     document.getElementById('login_container'));
```

### Other Pages

For user.html or other forms:

```html
<div id="change_password">
    <error-banner></error-banner>
    <h2>Change Password</h2>
    <!-- form content -->
</div>
```

```javascript
// Include the utility functions for easier usage
// <script src="/static/js/utils/error-utils.js"></script>

// Then use the utility functions:
showFormError("Passwords do not match", document.getElementById('change_password'));
```

## CSS Styling

The component uses Shadow DOM, so its styles are encapsulated. However, you can style the host element:

```css
error-banner {
    margin: -0.5rem -0.5rem 1rem -0.5rem; /* Adjust positioning within containers */
}
```

## Utility Functions

Include `error-utils.js` for additional helper functions:

```javascript
// General error
showErrorMessage("Something went wrong");

// Form validation error
showFormError("Invalid input", formContainer);

// API/Network error
showAPIError(error, container);

// Hide all error banners
hideAllErrors(container);
```

## Browser Support

- Chrome 54+
- Firefox 63+
- Safari 10.1+
- Edge 79+

The component uses Web Components (Custom Elements and Shadow DOM), which are supported in all modern browsers.
