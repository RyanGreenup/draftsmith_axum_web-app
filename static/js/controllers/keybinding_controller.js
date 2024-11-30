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
        if (event.ctrlKey) {
            switch (event.key.toLowerCase()) {
                case 'e':
                    console.log("Ctrl+E detected")
                    event.preventDefault()
                    const editButton = document.querySelector('a[href$="/edit"]')
                    if (editButton) {
                        console.log("Edit button found, clicking")
                        editButton.click()
                    }
                    break
                    
                case 'arrowleft':
                    event.preventDefault()
                    this.navigateToPage('prev')
                    break
                    
                case 'arrowright':
                    event.preventDefault()
                    this.navigateToPage('next')
                    break
                    
                case 'arrowup':
                    event.preventDefault()
                    this.navigateToNearestNote('prev')
                    break
                    
                case 'arrowdown':
                    event.preventDefault()
                    this.navigateToNearestNote('next')
                    break
            }
        }
    }

    navigateToPage(direction) {
        const urlParams = new URLSearchParams(window.location.search)
        const currentPage = parseInt(urlParams.get('page')) || 1
        
        let newPage
        if (direction === 'next') {
            newPage = currentPage + 1
        } else {
            newPage = Math.max(currentPage - 1, 1)
        }

        if (newPage !== currentPage) {
            const url = new URL(window.location.href)
            url.searchParams.set('page', newPage)
            window.location.href = url.toString()
        }
    }

    navigateToNearestNote(direction) {
        const noteLinks = document.querySelectorAll('.note-item a')
        if (!noteLinks.length) return

        const currentPath = window.location.pathname
        const currentNoteId = currentPath.split('/').pop()

        let currentIndex = -1
        noteLinks.forEach((link, index) => {
            if (link.href.endsWith(`/note/${currentNoteId}`)) {
                currentIndex = index
            }
        })

        let newIndex
        if (direction === 'next') {
            newIndex = currentIndex < noteLinks.length - 1 ? currentIndex + 1 : 0
        } else {
            newIndex = currentIndex > 0 ? currentIndex - 1 : noteLinks.length - 1
        }

        if (newIndex !== currentIndex) {
            window.location.href = noteLinks[newIndex].href
        }
    }
}
