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
    // Extract just the ID number from the href
    const tagId = tagItem.querySelector('a').href.split('/').pop()
    console.log('Dragging tag with ID:', tagId) // Debug logging

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

    console.log('Moving tag', draggedTagId, 'to parent', targetTagId) // Debug logging

    // Don't do anything if dropping on itself
    if (draggedTagId === targetTagId) {
        return
    }

    try {
        // Make the API call to set the new parent
        const response = await fetch(`/tag/${draggedTagId}/set_parent`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
                'Accept': 'application/json',
            },
            body: new URLSearchParams({
                'parent_id': targetTagId
            }).toString()
        })

        if (!response.ok) {
            const errorText = await response.text()
            throw new Error(`Move failed: ${errorText}`)
        }

        // Force a full page reload to show the updated hierarchy
        window.location.href = '/manage_tags'

    } catch (error) {
        console.error('Error moving tag:', error)
        alert('Failed to update tag hierarchy: ' + error.message)
        window.location.href = '/manage_tags'
    }
  }

  handleBodyDragOver(event) {
    // Get coordinates of the drop zone
    const dropZoneRect = {
        bottom: window.innerHeight - 20,
        top: window.innerHeight - 120,
        left: (window.innerWidth - 200) / 2,
        right: (window.innerWidth + 200) / 2
    };

    // Check if the drag is within the drop zone area
    const isInDropZone = 
        event.clientY >= dropZoneRect.top &&
        event.clientY <= dropZoneRect.bottom &&
        event.clientX >= dropZoneRect.left &&
        event.clientX <= dropZoneRect.right;

    // Only allow dropping in the specific drop zone area
    if (isInDropZone && !event.target.closest('.menu') && !event.target.closest('.drawer-side')) {
        event.preventDefault();
        document.body.classList.add('detach-drop-zone');
        if (!document.body.classList.contains('drag-hover')) {
            document.body.classList.add('drag-hover');
        }
    } else {
        document.body.classList.remove('detach-drop-zone', 'drag-hover');
    }
  }

  async handleBodyDrop(event) {
    const dropZoneRect = {
        bottom: window.innerHeight - 20,
        top: window.innerHeight - 120,
        left: (window.innerWidth - 200) / 2,
        right: (window.innerWidth + 200) / 2
    };

    const isInDropZone = 
        event.clientY >= dropZoneRect.top &&
        event.clientY <= dropZoneRect.bottom &&
        event.clientX >= dropZoneRect.left &&
        event.clientX <= dropZoneRect.right;

    if (isInDropZone && !event.target.closest('.menu')) {
        event.preventDefault();
        document.body.classList.remove('detach-drop-zone', 'drag-hover');

        const draggedTagId = event.dataTransfer.getData('text/plain');
        if (!draggedTagId) return;

        try {
            const response = await fetch(`/tag/${draggedTagId}/unset_parent`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                    'Accept': 'application/json',
                }
            });

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`Detach failed: ${errorText}`);
            }

            window.location.href = '/manage_tags';

        } catch (error) {
            console.error('Error detaching tag:', error);
            alert('Failed to detach tag: ' + error.message);
            window.location.href = '/manage_tags';
        }
    }
  }
}
