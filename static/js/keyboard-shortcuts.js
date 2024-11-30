class KeyboardShortcuts {
    constructor() {
        this.shortcuts = {
            // Navigation shortcuts
            prevPage: {
                key: 'ArrowLeft',
                modifier: 'Alt',
                action: () => this.navigateToPage('prev')
            },
            nextPage: {
                key: 'ArrowRight',
                modifier: 'Alt',
                action: () => this.navigateToPage('next')
            },
            prevNote: {
                key: 'ArrowUp',
                modifier: 'Alt',
                action: () => this.navigateToNearestNote('prev')
            },
            nextNote: {
                key: 'ArrowDown',
                modifier: 'Alt',
                action: () => this.navigateToNearestNote('next')
            },

            // Action shortcuts
            edit: {
                key: 'e',
                modifier: 'Alt',
                selector: 'a[data-edit-link], a[href$="/edit"]'
            },
            create: {
                key: 'c',
                modifier: 'Alt',
                selector: 'a[data-create-link]'
            },
            lambda: {
                key: ['Backquote', '`'],  // Accept both key names
                modifier: 'Alt',
                action: () => this.insertTextAtCaret('λ#()#')
            },
            submit: {
                key: 'Enter',
                modifier: 'Control',
                action: () => this.submitForm()
            }
        };
        
        this.init();
    }

    init() {
        document.addEventListener('keydown', (event) => this.handleKeydown(event));
        console.log("Keyboard shortcuts initialized");
    }

    handleKeydown(event) {
        for (const [name, shortcut] of Object.entries(this.shortcuts)) {
            if (
                (shortcut.modifier === 'Alt' && event.altKey) ||
                (shortcut.modifier === 'Control' && event.ctrlKey)
            ) {
                const keys = Array.isArray(shortcut.key) ? shortcut.key : [shortcut.key];
                if (keys.some(k => event.key === k || event.key.toLowerCase() === k.toLowerCase())) {
                    event.preventDefault();
                    
                    if (shortcut.action) {
                        shortcut.action();
                    } else if (shortcut.selector) {
                        this.navigateToLink(shortcut.selector);
                    }
                }
            }
        }
    }

    submitForm() {
        const form = document.getElementById('content-edit-form');
        if (form) {
            form.submit();
            return false;
        }
    }

    navigateToLink(selector) {
        const link = document.querySelector(selector);
        if (link) {
            window.location.href = link.href;
        }
    }

    insertTextAtCaret(text) {
        const activeElement = document.activeElement;
        if (!activeElement || !['INPUT', 'TEXTAREA'].includes(activeElement.tagName)) {
            console.warn('No input field or textarea is active.');
            return;
        }

        const startPos = activeElement.selectionStart;
        const endPos = activeElement.selectionEnd;
        const originalText = activeElement.value;
        
        activeElement.value = originalText.slice(0, startPos) + 
                            text + 
                            originalText.slice(endPos);

        const newCaretPosition = startPos + text.indexOf('(') + 1;
        activeElement.setSelectionRange(newCaretPosition, newCaretPosition);
        activeElement.focus();
    }

    navigateToPage(direction) {
        const urlParams = new URLSearchParams(window.location.search);
        const currentPage = parseInt(urlParams.get('page')) || 1;
        
        let newPage;
        if (direction === 'next') {
            newPage = currentPage + 1;
        } else {
            newPage = Math.max(currentPage - 1, 1);
        }

        if (newPage !== currentPage) {
            const url = new URL(window.location.href);
            url.searchParams.set('page', newPage);
            window.location.href = url.toString();
        }
    }

    navigateToNearestNote(direction) {
        const noteLinks = document.querySelectorAll('.note-item a');
        if (!noteLinks.length) return;

        const currentPath = window.location.pathname;
        const currentNoteId = currentPath.split('/').pop();

        let currentIndex = -1;
        noteLinks.forEach((link, index) => {
            if (link.href.endsWith(`/note/${currentNoteId}`)) {
                currentIndex = index;
            }
        });

        let newIndex;
        if (direction === 'next') {
            newIndex = currentIndex < noteLinks.length - 1 ? currentIndex + 1 : 0;
        } else {
            newIndex = currentIndex > 0 ? currentIndex - 1 : noteLinks.length - 1;
        }

        if (newIndex !== currentIndex) {
            window.location.href = noteLinks[newIndex].href;
        }
    }
}

// Initialize keyboard shortcuts when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new KeyboardShortcuts();
});
