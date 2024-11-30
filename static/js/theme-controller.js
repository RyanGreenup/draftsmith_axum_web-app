class ThemeController {
    constructor() {
        this.init();
    }

    init() {
        // Restore theme from localStorage on page load
        const savedTheme = localStorage.getItem('theme');
        if (savedTheme) {
            document.documentElement.setAttribute('data-theme', savedTheme);
            // Update radio button
            const radio = document.querySelector(`input[name="theme-dropdown"][value="${savedTheme}"]`);
            if (radio) {
                radio.checked = true;
            }
        }

        // Add event listeners to theme radio buttons
        document.querySelectorAll('.theme-controller').forEach(input => {
            input.addEventListener('change', (e) => {
                const theme = e.target.value;
                document.documentElement.setAttribute('data-theme', theme);
                localStorage.setItem('theme', theme);
            });
        });
    }
}

// Initialize theme controller when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new ThemeController();
});
