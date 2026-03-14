import { BrowserRouter } from "react-router-dom";
import { ThreePanel } from "../components/layout/ThreePanel";
import { CommandPalette } from "../components/layout/CommandPalette";
import { ToastContainer } from "../components/notifications/ToastContainer";

function App() {
  return (
    <BrowserRouter>
      <ThreePanel />
      <CommandPalette />
      <ToastContainer />
    </BrowserRouter>
  );
}

export default App;
