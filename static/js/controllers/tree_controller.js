import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    // Initialize all note items as draggable
    this.element.querySelectorAll('.note-item').forEach(item => {
      item.addEventListener('dragstart', this.handleDragStart.bind(this))
      item.addEventListener('dragover', this.handleDragOver.bind(this))
      item.addEventListener('drop', this.handleDrop.bind(this))
    })
  }

  handleDragStart(event) {
    // Store the dragged note's ID
    event.dataTransfer.setData('text/plain', event.target.dataset.noteId)
    event.target.classList.add('opacity-50')
  }

  handleDragOver(event) {
    // Prevent default to allow drop
    event.preventDefault()
    // Add visual feedback
    event.target.closest('.note-item').classList.add('bg-base-300')
  }

  handleDragEnd(event) {
    // Remove visual feedback
    event.target.classList.remove('opacity-50')
    this.element.querySelectorAll('.note-item').forEach(item => {
      item.classList.remove('bg-base-300')
    })
  }

  async handleDrop(event) {
    event.preventDefault()
    
    // Remove visual feedback
    event.target.closest('.note-item').classList.remove('bg-base-300')
    
    const draggedNoteId = event.dataTransfer.getData('text/plain')
    const targetNoteId = event.target.closest('.note-item').dataset.noteId
    
    // Don't do anything if dropping on itself
    if (draggedNoteId === targetNoteId) {
      return
    }

    try {
      // Make the API call to move the note
      const response = await fetch(`/note/${draggedNoteId}/move`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: `new_parent_id=${targetNoteId}`
      })

      if (!response.ok) {
        throw new Error('Move failed')
      }

      // Reload the page to show the updated tree
      window.location.reload()
    } catch (error) {
      console.error('Error moving note:', error)
      // Show error message to user
      const flash = document.getElementById('flash-messages')
      if (flash) {
        flash.innerHTML = `
          <div class="alert alert-error">
            <span>Failed to move note. Please try again.</span>
          </div>
        `
      }
    }
  }
}
