import { useState } from 'react';
import Versions from './components/Versions';
import EventsDemo from './components/EventsDemo';
import electronLogo from './assets/electron.svg';
import wavyLines from './assets/wavy-lines.svg';

function App(): JSX.Element {
  const [showEventsDemo, setShowEventsDemo] = useState(false);

  return (
    <div className="app-container">
      <header className="app-header">
        <img alt="logo" className="logo" src={electronLogo} />
        <h1>Zelan</h1>
        <p className="tagline">Stream Data Aggregation Service</p>
      </header>
      
      <div className="content-container">
        <div className="sidebar">
          <nav>
            <ul>
              <li>
                <button 
                  className={!showEventsDemo ? 'active' : ''}
                  onClick={() => setShowEventsDemo(false)}
                >
                  Dashboard
                </button>
              </li>
              <li>
                <button 
                  className={showEventsDemo ? 'active' : ''}
                  onClick={() => setShowEventsDemo(true)}
                >
                  Events Demo
                </button>
              </li>
            </ul>
          </nav>
        </div>
        
        <main className="main-content">
          {showEventsDemo ? (
            <EventsDemo />
          ) : (
            <div className="dashboard">
              <h2>Dashboard</h2>
              <p>Welcome to Zelan, your stream data aggregation service.</p>
              <p>Click on &quot;Events Demo&quot; to see the reactive event system in action.</p>
              <Versions />
            </div>
          )}
        </main>
      </div>
      
      <img src={wavyLines} className="background-decoration" alt="" aria-hidden="true" />
      
      <style>{`
        .app-container {
          display: flex;
          flex-direction: column;
          height: 100vh;
          overflow: hidden;
        }
        
        .app-header {
          background: #1e1e1e;
          color: white;
          padding: 15px 20px;
          display: flex;
          align-items: center;
          border-bottom: 1px solid #333;
        }
        
        .app-header .logo {
          width: 36px;
          height: 36px;
          margin-right: 15px;
        }
        
        .app-header h1 {
          margin: 0;
          font-size: 24px;
          font-weight: 600;
        }
        
        .tagline {
          margin: 0 0 0 15px;
          opacity: 0.7;
          font-size: 14px;
        }
        
        .content-container {
          display: flex;
          flex: 1;
          overflow: hidden;
        }
        
        .sidebar {
          width: 220px;
          background: #252525;
          border-right: 1px solid #333;
          padding: 20px 0;
        }
        
        .sidebar nav ul {
          list-style: none;
          padding: 0;
          margin: 0;
        }
        
        .sidebar nav li {
          margin-bottom: 5px;
        }
        
        .sidebar nav button {
          width: 100%;
          text-align: left;
          padding: 10px 20px;
          background: none;
          border: none;
          color: #ccc;
          font-size: 15px;
          cursor: pointer;
          border-left: 3px solid transparent;
        }
        
        .sidebar nav button:hover {
          background: rgba(255, 255, 255, 0.05);
        }
        
        .sidebar nav button.active {
          color: white;
          background: rgba(255, 255, 255, 0.1);
          border-left-color: #0078d4;
        }
        
        .main-content {
          flex: 1;
          padding: 20px;
          overflow-y: auto;
          position: relative;
        }
        
        .dashboard {
          max-width: 800px;
          margin: 0 auto;
        }
        
        .background-decoration {
          position: absolute;
          bottom: 0;
          right: 0;
          width: 80%;
          z-index: -1;
          opacity: 0.05;
          pointer-events: none;
        }
      `}</style>
    </div>
  );
}

export default App;
