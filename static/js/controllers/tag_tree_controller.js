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

        // Initialize detach zone
        const detachZone = this.element.querySelector('.detach-zone')
        if (detachZone) {
            detachZone.addEventListener('dragover', this.handleDetachZoneDragOver.bind(this))
            detachZone.addEventListener('dragleave', this.handleDetachZoneDragLeave.bind(this))
            detachZone.addEventListener('drop', this.handleDetachZoneDrop.bind(this))
        }
    }

    handleDragStart(event) {
        const tagItem = event.target.closest('.collapse-title')
        const tagId = tagItem.querySelector('a').href.split('/').pop()
        console.log('Dragging tag with ID:', tagId)

        event.dataTransfer.setData('text/plain', tagId)
        tagItem.classList.add('dragging')
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
        this.element.querySelectorAll('.collapse-title').forEach(item => {
            item.classList.remove('dragging', 'bg-base-300')
        })
        const detachZone = this.element.querySelector('.detach-zone')
        if (detachZone) {
            detachZone.classList.remove('drag-hover')
        }
    }

    async handleDrop(event) {
        event.preventDefault()

        const targetItem = event.target.closest('.collapse-title')
        if (!targetItem) return

        targetItem.classList.remove('bg-base-300')

        const draggedTagId = event.dataTransfer.getData('text/plain')
        const targetTagId = targetItem.querySelector('a').href.split('/').pop()

        if (draggedTagId === targetTagId) {
            return
        }

        try {
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

            window.location.href = '/manage_tags'

        } catch (error) {
            console.error('Error moving tag:', error)
            alert('Failed to update tag hierarchy: ' + error.message)
            window.location.href = '/manage_tags'
        }
    }

    handleDetachZoneDragOver(event) {
        event.preventDefault()
        event.currentTarget.classList.add('drag-hover')
    }

    handleDetachZoneDragLeave(event) {
        event.currentTarget.classList.remove('drag-hover')
    }

    async handleDetachZoneDrop(event) {
        event.preventDefault()
        event.currentTarget.classList.remove('drag-hover')

        const draggedTagId = event.dataTransfer.getData('text/plain')
        if (!draggedTagId) return

        try {
            const response = await fetch(`/tag/${draggedTagId}/unset_parent`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                    'Accept': 'application/json',
                }
            })

            if (!response.ok) {
                const errorText = await response.text()
                throw new Error(`Detach failed: ${errorText}`)
            }

            window.location.href = '/manage_tags'

        } catch (error) {
            console.error('Error detaching tag:', error)
            alert('Failed to detach tag: ' + error.message)
            window.location.href = '/manage_tags'
        }
    }
}
