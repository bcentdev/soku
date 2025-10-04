import styles from './src/Button.module.css';

console.log('Button styles:', styles);

const button = document.createElement('button');
button.className = styles.button;
button.textContent = 'Click me';
document.body.appendChild(button);
