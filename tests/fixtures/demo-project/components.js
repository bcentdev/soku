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
        header.innerHTML = '<h1>Ultra Bundler Test</h1>';
        document.body.appendChild(header);
    },

    createContent() {
        const main = document.createElement('main');
        main.innerHTML = `
            <div class="content">
                <p>This is a clean test project for Ultra Bundler</p>
                <button onclick="alert('Ultra fast!')">Test Button</button>
            </div>
        `;
        document.body.appendChild(main);
    },

    createFooter() {
        const footer = document.createElement('footer');
        footer.innerHTML = '<p>&copy; 2024 Ultra Bundler</p>';
        document.body.appendChild(footer);
    }
};