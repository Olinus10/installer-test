document.addEventListener('DOMContentLoaded', function() {
  // Create the particles container if it doesn't exist
  if (!document.querySelector('.particles-container')) {
    const container = document.createElement('div');
    container.className = 'particles-container';
    document.body.prepend(container); // Add to the beginning of body
  }
  
  const particlesContainer = document.querySelector('.particles-container');
  const particleCount = 50; // Adjust number of particles
  
  // Clear any existing particles
  particlesContainer.innerHTML = '';
  
  // Create particles
  for (let i = 0; i < particleCount; i++) {
    createParticle(particlesContainer);
  }
});

function createParticle(container) {
  // Create a new particle element
  const particle = document.createElement('div');
  particle.className = 'particle';
  
  // Randomly select particle type (for color variety)
  const types = ['', 'purple', 'green'];
  const randomType = types[Math.floor(Math.random() * types.length)];
  if (randomType) {
    particle.classList.add(randomType);
  }
  
  // Random size (pixels)
  const size = Math.random() * 7 + 3; // Between 3-10px
  
  // Random position
  const posX = Math.random() * 100; // % of viewport width
  const posY = Math.random() * 100 + 100; // % of viewport height (start below viewport)
  
  // Random opacity
  const opacity = Math.random() * 0.3 + 0.1; // Between 0.1-0.4
  
  // Random animation duration
  const duration = Math.random() * 30 + 20; // Between 20-50 seconds
  
  // Random animation delay
  const delay = Math.random() * 15; // Between 0-15 seconds
  
  // Choose between the two animations
  const animation = Math.random() > 0.3 ? 'float' : 'float-horizontal';
  
  // Apply styles
  particle.style.width = `${size}px`;
  particle.style.height = `${size}px`;
  particle.style.left = `${posX}%`;
  particle.style.top = `${posY}%`;
  particle.style.opacity = opacity;
  particle.style.animation = `${animation} ${duration}s linear ${delay}s infinite`;
  
  // Add to container
  container.appendChild(particle);
  
  // Remove and recreate particle when animation completes one cycle
  // This ensures particles don't all disappear at once after first cycle
  setTimeout(() => {
    particle.remove();
    createParticle(container);
  }, (duration + delay) * 1000);
}
