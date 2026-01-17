import '@testing-library/jest-dom/vitest';

const noop = () => {};

class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

class IntersectionObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: noop,
    removeListener: noop,
    addEventListener: noop,
    removeEventListener: noop,
    dispatchEvent: () => false,
  }),
});

Object.defineProperty(window, 'scrollTo', { value: noop, writable: true });

Object.defineProperty(globalThis, 'ResizeObserver', {
  value: ResizeObserver,
  writable: true,
});

Object.defineProperty(globalThis, 'IntersectionObserver', {
  value: IntersectionObserver,
  writable: true,
});
