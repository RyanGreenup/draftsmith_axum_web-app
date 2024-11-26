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

    // Add drop zone for detaching notes
    document.body.addEventListener('dragover', this.handleBodyDragOver.bind(this))
    document.body.addEventListener('drop', this.handleBodyDrop.bind(this))
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
    document.body.classList.remove('detach-drop-zone')
  }

  async handleDrop(event) {
    event.preventDefault();
    
    const targetItem = event.target.closest('.note-item');
    if (!targetItem) return;
    
    // Remove visual feedback
    targetItem.classList.remove('drag-over');
    
    const draggedNoteId = event.dataTransfer.getData('text/plain');
    const targetNoteId = targetItem.dataset.noteId;
    
    // Don't do anything if dropping on itself
    if (draggedNoteId === targetNoteId) {
        return;
    }

    try {
        // Make the API call to move the note
        const response = await fetch(`/note/${draggedNoteId}/move`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            // Properly format the form data
            body: new URLSearchParams({
                'new_parent_id': targetNoteId
            }).toString()
        });

        if (!response.ok) {
            throw new Error(`Move failed: ${response.statusText}`);
        }

        // Redirect to the note's page to show the result and flash message
        window.location.href = `/note/${draggedNoteId}`;
        
    } catch (error) {
        console.error('Error moving note:', error);
        // Force a page reload to show any error flash messages
        window.location.href = `/note/${draggedNoteId}`;
    }
  }

  handleBodyDragOver(event) {
    // Only allow dropping outside the tree
    if (!event.target.closest('.note-tree')) {
        event.preventDefault()
        document.body.classList.add('detach-drop-zone')
    }
  }

  async handleBodyDrop(event) {
    // Only handle drops outside the tree
    if (!event.target.closest('.note-tree')) {
        event.preventDefault()
        document.body.classList.remove('detach-drop-zone')
        
        const draggedNoteId = event.dataTransfer.getData('text/plain')
        if (!draggedNoteId) return

        try {
            // Make the API call to detach the note
            const response = await fetch(`/note/${draggedNoteId}/move`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                },
                body: new URLSearchParams({
                    'new_parent_id': '0'  // Use 0 or null to indicate detachment
                }).toString()
            })

            if (!response.ok) {
                throw new Error(`Detach failed: ${response.statusText}`)
            }

            // Redirect to the note's page to show the result
            window.location.href = `/note/${draggedNoteId}`
            
        } catch (error) {
            console.error('Error detaching note:', error)
            window.location.href = `/note/${draggedNoteId}`
        }
    }
  }
}
