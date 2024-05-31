const eventSource = new EventSource('/stream');

eventSource.addEventListener('message', (event) => {
    const data = JSON.parse(event.data);
    const { ip, timestamp, container_name, content } = data;
    console.log(data);
    const entries = document.getElementById('entries');
    const newEntry = document.createElement('div');
    newEntry.textContent = `${container_name} - ${timestamp} - ${content}`;
    newEntry.classList.add('log-entry', 'highlight');
    entries.prepend(newEntry);
    setTimeout(() => {
        newEntry.classList.remove('highlight');
    }, 2000);
});

eventSource.addEventListener('error', (error) => {
    console.error('An error occurred:', error);
});

const hrElement = document.createElement('hr');
document.body.prepend(hrElement);

const rainbowText = document.createElement('h1');
rainbowText.textContent = 'Unicorn Logs';
rainbowText.classList.add('rainbow-text');
document.body.prepend(rainbowText);


setInterval(() => {
    const colors = ['red', 'orange', 'yellow', 'green', 'blue', 'indigo', 'violet'];
    const letters = rainbowText.textContent.split('');
    const coloredLetters = letters.map((letter, index) => {
        const colorIndex = Math.floor(Math.random() * colors.length);
        const nextColorIndex = (colorIndex + 1) % colors.length;
        return `<span style="color: ${colors[nextColorIndex]}">${letter}</span>`;
    });
    rainbowText.innerHTML = coloredLetters.join('');
}, 1000);