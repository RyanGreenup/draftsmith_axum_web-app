import { Controller } from "/static/js/stimulus/stimulus.js"

export default class extends Controller {
  connect() {
    console.log("TagTreeController connected");

    // Track original states of details elements
    this.originalDetailsStates = new WeakMap()

    // Initialize all tag items as draggable
    this.element.querySelectorAll('.collapse-title').forEach(item => {
      item.setAttribute('draggable', true)
      item.addEventListener('dragstart', this.handleDragStart.bind(this))
      item.addEventListener('dragover', this.handleDragOver.bind(this))
      item.addEventListener('drop', this.handleDrop.bind(this))
      item.addEventListener('dragend', this.handleDragEnd.bind(this))
      item.addEventListener('dragleave', this.handleDragLeave.bind(this))
    })

    // Add drop zone for detaching tags
    document.body.addEventListener('dragover', this.handleBodyDragOver.bind(this))
    document.body.addEventListener('drop', this.handleBodyDrop.bind(this))
  }

  handleDragStart(event) {
    const tagItem = event.target.closest('.collapse-title')
    const tagId = tagItem.querySelector('a').href.split('/').pop()
    // Store the dragged tag's ID
    event.dataTransfer.setData('text/plain', tagId)
    // Add dragging class for visual feedback
    tagItem.classList.add('dragging')
    // Set the drag effect
    event.dataTransfer.effectAllowed = 'move'
  }

  handleDragOver(event) {
    event.preventDefault()
    const tagItem = event.target.closest('.collapse-title')
    if (tagItem) {
      tagItem.classList.add('bg-base-300')
    }
  }

  handleDragLeave(event) {
    const tagItem = event.target.closest('.collapse-title')
    if (tagItem) {
      tagItem.classList.remove('bg-base-300')
    }
  }

  handleDragEnd(event) {
    // Remove all drag-related classes
    this.element.querySelectorAll('.collapse-title').forEach(item => {
      item.classList.remove('dragging', 'bg-base-300')
    })
    document.body.classList.remove('detach-drop-zone')
  }

  async handleDrop(event) {
    event.preventDefault()

    const targetItem = event.target.closest('.collapse-title')
    if (!targetItem) return

    // Remove visual feedback
    targetItem.classList.remove('bg-base-300')

    const draggedTagId = event.dataTransfer.getData('text/plain')
    const targetTagId = targetItem.querySelector('a').href.split('/').pop()

    // Don't do anything if dropping on itself
    if (draggedTagId === targetTagId) {
        return
    }

    try {
        // Make a single request to set the new parent
        const response = await fetch(`/tags/${draggedTagId}/set_parent`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: new URLSearchParams({
                'parent_id': targetTagId
            }).toString()
        })

        if (!response.ok) {
            throw new Error(`Move failed: ${response.statusText}`)
        }

        // Reload the page to show the updated hierarchy
        window.location.reload()

    } catch (error) {
        console.error('Error moving tag:', error)
        window.location.reload()
    }
  }

  handleBodyDragOver(event) {
    // Only allow dropping outside the tree
    if (!event.target.closest('.menu')) {
      event.preventDefault()
      document.body.classList.add('detach-drop-zone')
    }
  }

  async handleBodyDrop(event) {
    // Only handle drops outside the tree
    if (!event.target.closest('.menu')) {
      event.preventDefault()
      document.body.classList.remove('detach-drop-zone')

      const draggedTagId = event.dataTransfer.getData('text/plain')
      if (!draggedTagId) return

      try {
        // Make the API call to detach the tag
        const response = await fetch(`/tags/${draggedTagId}/set_parent`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/x-www-form-urlencoded',
          },
          body: new URLSearchParams({
            'parent_id': ''  // Empty string to indicate detachment
          }).toString()
        })

        if (!response.ok) {
          throw new Error(`Detach failed: ${response.statusText}`)
        }

        // Reload the page to show the updated hierarchy
        window.location.reload()

      } catch (error) {
        console.error('Error detaching tag:', error)
        window.location.reload()
      }
    }
  }
}
