import React from 'react';
import ReactDOM from 'react-dom/client';
import '@fontsource/inter/latin-400.css';
import '@fontsource/inter/latin-500.css';
import '@fontsource/inter/latin-600.css';
import '@fontsource/jetbrains-mono/latin-400.css';
import 'dockview/dist/styles/dockview.css';
import './style.css';
import { App } from './App';
// Middle-dragging in a 3D view pans the camera; stop the browser's middle-click autoscroll from also scrolling
// the page.
document.addEventListener(
  'mousedown',
  (e) => {
    if (e.button === 1 && e.target instanceof HTMLCanvasElement) e.preventDefault();
  },
  { capture: true },
);
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
