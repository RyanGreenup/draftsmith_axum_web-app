import { Controller } from "/static/js/stimulus/stimulus.js"

export default class extends Controller {
    connect() {
        console.log("KeybindingController connected")
        // Use window instead of document to ensure global capture
        window.addEventListener('keydown', this.handleKeyPress.bind(this))
    }

    disconnect() {
        window.removeEventListener('keydown', this.handleKeyPress.bind(this))
    }

    handleKeyPress(event) {
        console.log("Key pressed:", {
            key: event.key,
            ctrlKey: event.ctrlKey,
            altKey: event.altKey
        });

        // Check for Ctrl+E (case-insensitive)
        if (event.ctrlKey && (event.key.toLowerCase() === 'e')) {
            console.log("Ctrl+E detected");
            event.preventDefault();
            const editButton = document.querySelector('a[href$="/edit"]');
            if (editButton) {
                console.log("Edit button found, clicking");
                editButton.click();
            } else {
                console.log("No edit button found");
            }
        }

        // Ctrl+Enter to submit
        if (event.ctrlKey && event.key === 'Enter') {
            event.preventDefault()
            const submitButton = document.querySelector('form button[type="submit"]')
            if (submitButton) {
                submitButton.click()
            }
        }
    }
}
