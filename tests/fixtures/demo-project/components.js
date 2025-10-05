// Component system
export const components = {
    render() {
        console.log('Rendering components');
        this.createHeader();
        this.createContent();
        this.createFooter();
    },

    createHeader() {
        const header = document.createElement('header');
        header.innerHTML = '<h1>Soku Bundler Test</h1>';
        document.body.appendChild(header);
    },

    createContent() {
        const main = document.createElement('main');
        main.innerHTML = `
            <div class="content">
                <p>This is a clean test project for Soku Bundler</p>
                <button onclick="alert('Lightning fast!')">Test Button</button>
            </div>
        `;
        document.body.appendChild(main);
    },

    createFooter() {
        const footer = document.createElement('footer');
        footer.innerHTML = '<p>&copy; 2024 Soku Bundler</p>';
        document.body.appendChild(footer);
    }
};