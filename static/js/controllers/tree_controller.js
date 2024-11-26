import { Controller } from "/static/js/stimulus/stimulus.js"

export default class extends Controller {
  connect() {
    console.log("TreeController connected");
    
    // Initialize all note items as draggable
    this.element.querySelectorAll('.note-item').forEach(item => {
      console.log("Setting up draggable item:", item);
      item.addEventListener('dragstart', this.handleDragStart.bind(this))
      item.addEventListener('dragover', this.handleDragOver.bind(this))
      item.addEventListener('drop', this.handleDrop.bind(this))
      item.addEventListener('dragend', this.handleDragEnd.bind(this))
      item.addEventListener('dragleave', this.handleDragLeave.bind(this))
    })
  }

  handleDragStart(event) {
    const noteItem = event.target.closest('.note-item')
    // Store the dragged note's ID
    event.dataTransfer.setData('text/plain', noteItem.dataset.noteId)
    // Add dragging class for visual feedback
    noteItem.classList.add('dragging')
    // Set the drag effect
    event.dataTransfer.effectAllowed = 'move'
  }

  handleDragOver(event) {
    // Prevent default to allow drop
    event.preventDefault()
    const noteItem = event.target.closest('.note-item')
    if (noteItem) {
      // Add drag-over class for visual feedback
      noteItem.classList.add('drag-over')
    }
  }

  handleDragLeave(event) {
    const noteItem = event.target.closest('.note-item')
    if (noteItem) {
      // Remove drag-over class when leaving
      noteItem.classList.remove('drag-over')
    }
  }

  handleDragEnd(event) {
    // Remove all drag-related classes
    this.element.querySelectorAll('.note-item').forEach(item => {
      item.classList.remove('dragging', 'drag-over')
    })
  }

  async handleDrop(event) {
    event.preventDefault()
    
    const targetItem = event.target.closest('.note-item')
    if (!targetItem) return
    
    // Remove visual feedback
    targetItem.classList.remove('drag-over')
    
    const draggedNoteId = event.dataTransfer.getData('text/plain')
    const targetNoteId = targetItem.dataset.noteId
    
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
