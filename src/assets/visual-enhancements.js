document.addEventListener('DOMContentLoaded', function() {
  // Initialize all visual enhancements
  initParticlesBackground();
  initEnhancedCards();
  initInstallButton();
  setupProgressBar();
});

// ======= Particles Background =======
function initParticlesBackground() {
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
}

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

// ======= Enhanced Cards =======
function initEnhancedCards() {
  // Find all modpack cards
  const modpackCards = document.querySelectorAll('.home-pack-card');
  
  modpackCards.forEach(card => {
    // Get modpack data from card attributes
    const category = card.getAttribute('data-category') || 'gameplay';
    const version = card.getAttribute('data-version') || '1.0.0';
    const isNew = card.getAttribute('data-new') === 'true';
    const isUpdated = card.getAttribute('data-updated') === 'true';
    const modsCount = card.getAttribute('data-mods-count') || '0';
    const description = card.getAttribute('data-description') || '';
    
    // Add category badge
    if (!card.querySelector('.category-badge')) {
      const categoryBadge = document.createElement('div');
      categoryBadge.className = `category-badge ${category.toLowerCase()}`;
      categoryBadge.textContent = category;
      card.appendChild(categoryBadge);
    }
    
    // Add version badge
    if (!card.querySelector('.version-badge')) {
      const versionBadge = document.createElement('div');
      versionBadge.className = 'version-badge';
      versionBadge.textContent = `v${version}`;
      card.appendChild(versionBadge);
    }
    
    // Add NEW ribbon if applicable
    if (isNew && !card.querySelector('.new-ribbon')) {
      const newRibbon = document.createElement('div');
      newRibbon.className = 'new-ribbon';
      newRibbon.textContent = 'NEW';
      card.appendChild(newRibbon);
    }
    
    // Add UPDATED ribbon if applicable
    if (isUpdated && !card.querySelector('.updated-ribbon') && !isNew) {
      const updatedRibbon = document.createElement('div');
      updatedRibbon.className = 'updated-ribbon';
      updatedRibbon.textContent = 'UPDATED';
      card.appendChild(updatedRibbon);
    }
    
    // Add mods count indicator
    if (!card.querySelector('.mods-count')) {
      const modsCountEl = document.createElement('div');
      modsCountEl.className = 'mods-count';
      modsCountEl.textContent = `${modsCount} mods`;
      card.appendChild(modsCountEl);
    }
    
    // Add description to info section if applicable
    const infoSection = card.querySelector('.home-pack-info');
    if (infoSection && description && !infoSection.querySelector('.home-pack-description')) {
      const descriptionEl = document.createElement('div');
      descriptionEl.className = 'home-pack-description';
      descriptionEl.textContent = description;
      
      // Insert after title
      const title = infoSection.querySelector('.home-pack-title');
      if (title && title.nextSibling) {
        infoSection.insertBefore(descriptionEl, title.nextSibling);
      } else {
        infoSection.appendChild(descriptionEl);
      }
    }
  });
}

// ======= Install Button Enhancements =======
function initInstallButton() {
  const buttons = document.querySelectorAll('.main-install-button');
  
  buttons.forEach(button => {
    // Add button text wrapper if it doesn't exist
    if (!button.querySelector('.button-text')) {
      const textContent = button.textContent.trim();
      button.innerHTML = ''; // Clear current content
      
      // Add icon if applicable
      const icon = document.createElement('span');
      icon.className = 'button-icon';
      button.appendChild(icon);
      
      // Add text wrapper
      const text = document.createElement('span');
      text.className = 'button-text';
      text.textContent = textContent;
      button.appendChild(text);
      
      // Add progress indicator
      const progress = document.createElement('div');
      progress.className = 'button-progress';
      button.appendChild(progress);
    }
    
    // Create particle effect
    createButtonParticles(button);
  });
}

function createButtonParticles(button) {
  const wrapper = button.closest('.button-scale-wrapper');
  if (!wrapper) return;
  
  // Create particles container if it doesn't exist
  let particlesContainer = wrapper.querySelector('.button-particles');
  if (!particlesContainer) {
    particlesContainer = document.createElement('div');
    particlesContainer.className = 'button-particles';
    wrapper.appendChild(particlesContainer);
  }
  
  // Create particles on hover
  wrapper.addEventListener('mouseenter', () => {
    // Clear existing particles
    particlesContainer.innerHTML = '';
    
    // Create new particles
    for (let i = 0; i < 15; i++) {
      createButtonParticle(particlesContainer);
    }
  });
}

function createButtonParticle(container) {
  const particle = document.createElement('div');
  particle.className = 'button-particle';
  
  // Random position around the button
  const posX = Math.random() * 100;
  const posY = Math.random() * 100;
  
  // Random animation variables
  const tx = (Math.random() - 0.5) * 150; // -75px to 75px
  const ty = (Math.random() - 0.5) * 150; // -75px to 75px
  const duration = Math.random() * 1.5 + 1; // 1-2.5 seconds
  const delay = Math.random() * 0.5; // 0-0.5 seconds
  
  // Apply styles
  particle.style.left = `${posX}%`;
  particle.style.top = `${posY}%`;
  particle.style.width = `${Math.random() * 6 + 3}px`;
  particle.style.height = particle.style.width;
  particle.style.opacity = '0';
  particle.style.setProperty('--tx', `${tx}px`);
  particle.style.setProperty('--ty', `${ty}px`);
  
  // Different colors
  const colors = [
    'rgba(7, 60, 23, 0.6)',
    'rgba(10, 80, 30, 0.6)',
    'rgba(15, 100, 40, 0.6)'
  ];
  particle.style.backgroundColor = colors[Math.floor(Math.random() * colors.length)];
  
  // Add to container
  container.appendChild(particle);
  
  // Trigger animation in the next frame for proper rendering
  setTimeout(() => {
    particle.style.animation = `button-particle ${duration}s ease-out ${delay}s forwards`;
  }, 10);
  
  // Remove particle when animation completes
  setTimeout(() => {
    particle.remove();
  }, (duration + delay) * 1000);
}

// ======= Progress Bar Setup =======
function setupProgressBar() {
  const progressContainers = document.querySelectorAll('.progress-container');
  
  progressContainers.forEach(container => {
    // Get progress parameters
    const value = parseInt(container.getAttribute('data-value') || 0);
    const max = parseInt(container.getAttribute('data-max') || 100);
    const percentage = Math.min(Math.round((value / max) * 100), 100);
    const currentStep = container.getAttribute('data-step') || '';
    
    // Update linear progress bar
    const progressBar = container.querySelector('.progress-bar');
    if (progressBar) {
      progressBar.style.width = `${percentage}%`;
    }
    
    // Update percentage display
    const percentageEl = container.querySelector('.progress-percentage');
    if (percentageEl) {
      percentageEl.textContent = `${percentage}%`;
    }
    
    // Update steps indicators
    const steps = container.querySelectorAll('.progress-step');
    steps.forEach(step => {
      const stepId = step.getAttribute('data-step-id');
      
      // Check if this step is active or completed
      if (stepId === currentStep) {
        step.classList.add('active');
        step.classList.remove('completed');
      } else if (
        steps[0].getAttribute('data-step-id') === stepId && percentage === 0
      ) {
        // First step is active if progress is 0
        step.classList.add('active');
        step.classList.remove('completed');
      } else {
        const stepIndex = Array.from(steps).findIndex(s => s.getAttribute('data-step-id') === stepId);
        const currentIndex = Array.from(steps).findIndex(s => s.getAttribute('data-step-id') === currentStep);
        
        if (stepIndex < currentIndex || (percentage === 100 && currentStep === '')) {
          step.classList.add('completed');
          step.classList.remove('active');
        } else {
          step.classList.remove('active', 'completed');
        }
      }
    });
  });
}
