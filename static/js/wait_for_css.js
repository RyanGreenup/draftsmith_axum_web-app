function waitForStylesheets() {
  return Promise.all(
    Array.from(document.styleSheets).map(stylesheet => {
      if (stylesheet.href) {
        return new Promise((resolve, reject) => {
          const link = document.querySelector(`link[href="${stylesheet.href}"]`);
          if (!link) {
            resolve();
            return;
          }
          if (stylesheet.cssRules !== null) {
            resolve();
            return;
          }
          link.addEventListener('load', resolve);
          link.addEventListener('error', reject);
        });
      }
      return Promise.resolve();
    })
  );
}

// Initially hide content
document.documentElement.style.visibility = 'hidden';

// Show content once stylesheets are loaded
waitForStylesheets().then(() => {
  document.documentElement.style.visibility = 'visible';
}).catch(error => {
  console.error('Error loading stylesheets:', error);
  // Show content anyway in case of error
  document.documentElement.style.visibility = 'visible';
});
