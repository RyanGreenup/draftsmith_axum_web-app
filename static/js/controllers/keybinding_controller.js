import { Controller } from "/static/js/stimulus/stimulus.js"

export default class extends Controller {
    connect() {
        console.log("KeybindingController connected")
        document.addEventListener('keydown', this.handleKeyPress.bind(this))
    }

    disconnect() {
        document.removeEventListener('keydown', this.handleKeyPress.bind(this))
    }

    handleKeyPress(event) {
        // Ctrl+E to edit
        if (event.ctrlKey && event.key === 'e') {
            event.preventDefault()
            const editButton = document.querySelector('a[href$="/edit"]')
            if (editButton) {
                editButton.click()
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
