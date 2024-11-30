class KeyboardShortcuts {
    constructor() {
        this.shortcuts = {
            // Help shortcut
            help: {
                key: '?',
                modifier: 'Alt',
                action: () => this.showShortcutsHelp()
            },
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

    showShortcutsHelp() {
        // Create help dialog HTML
        const helpContent = `
            <dialog id="shortcuts-help" class="modal modal-bottom sm:modal-middle">
                <div class="modal-box">
                    <h3 class="font-bold text-lg mb-4">Keyboard Shortcuts</h3>
                    <table class="table table-zebra w-full">
                        <thead>
                            <tr>
                                <th>Action</th>
                                <th>Shortcut</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${this.generateShortcutsList()}
                        </tbody>
                    </table>
                    <div class="modal-action">
                        <form method="dialog">
                            <button class="btn">Close</button>
                        </form>
                    </div>
                </div>
                <form method="dialog" class="modal-backdrop">
                    <button>close</button>
                </form>
            </dialog>
        `;

        // Remove existing dialog if present
        const existingDialog = document.getElementById('shortcuts-help');
        if (existingDialog) {
            existingDialog.remove();
        }

        // Add dialog to document
        document.body.insertAdjacentHTML('beforeend', helpContent);

        // Show the dialog
        const dialog = document.getElementById('shortcuts-help');
        dialog.showModal();
    }

    generateShortcutsList() {
        return Object.entries(this.shortcuts)
            .map(([name, shortcut]) => {
                const keyName = Array.isArray(shortcut.key) 
                    ? shortcut.key[0] 
                    : shortcut.key;
                const displayKey = this.formatKeyName(keyName);
                const displayName = this.formatActionName(name);
                return `
                    <tr>
                        <td>${displayName}</td>
                        <td>${shortcut.modifier}+${displayKey}</td>
                    </tr>
                `;
            })
            .join('');
    }

    formatKeyName(key) {
        const specialKeys = {
            'ArrowLeft': '←',
            'ArrowRight': '→',
            'ArrowUp': '↑',
            'ArrowDown': '↓',
            'Backquote': '`'
        };
        return specialKeys[key] || key;
    }

    formatActionName(name) {
        return name
            .split(/(?=[A-Z])/)
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join(' ');
    }
}

// Initialize keyboard shortcuts when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new KeyboardShortcuts();
});
