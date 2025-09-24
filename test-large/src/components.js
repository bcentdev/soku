// Component system
export const components = {
    render() {
        console.log('Components rendering');
        this.createHeader();
        this.createMain();
        this.createFooter();
    },

    createHeader() {
        const header = document.createElement('header');
        header.innerHTML = '<h1>Ultra Bundler Test</h1>';
        document.body.appendChild(header);
    },

    createMain() {
        const main = document.createElement('main');
        main.innerHTML = `
            <div class="container">
                <p>This is a larger project to test auto-detection</p>
                <button id="test-btn">Test Button</button>
            </div>
        `;
        document.body.appendChild(main);

        // Add event listeners
        const btn = main.querySelector('#test-btn');
        btn.addEventListener('click', () => {
            console.log('Button clicked!');
        });
    },

    createFooter() {
        const footer = document.createElement('footer');
        footer.innerHTML = '<p>&copy; 2024 Ultra Bundle Test</p>';
        document.body.appendChild(footer);
    }
};