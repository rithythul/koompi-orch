import { ThreePanel } from "../components/layout/ThreePanel";
import { CommandPalette } from "../components/layout/CommandPalette";
import { ToastContainer } from "../components/notifications/ToastContainer";

function App() {
  return (
    <>
      <ThreePanel />
      <CommandPalette />
      <ToastContainer />
    </>
  );
}

export default App;
