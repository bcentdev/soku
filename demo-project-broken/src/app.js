// App components - Fixed
export const components = {
    render() {
        console.log('Components rendering');
        this.createHeader();
        this.createContent();
        this.attachEvents();
    },

    createHeader() {
        const header = document.createElement('header');
        header.innerHTML = '<h1>Ultra Bundler Demo</h1>';
        document.body.appendChild(header);
    },

    createContent() {
        const main = document.createElement('main');
        main.innerHTML = `
            <div class="content">
                <p>This demonstrates Ultra Bundler's capabilities</p>
                <button id="demo-btn">Test Button</button>
            </div>
        `;
        document.body.appendChild(main);
    },

    attachEvents() {
        const button = document.getElementById('demo-btn');
        if (button) {
            button.addEventListener('click', () => {
                console.log('🎯 Button clicked!');
                alert('Ultra Bundler works perfectly!');
            });
        }
    }
};