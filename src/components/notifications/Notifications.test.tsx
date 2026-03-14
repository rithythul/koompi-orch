import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent, act } from "@testing-library/react";
import { Toast } from "./Toast";
import { ToastContainer } from "./ToastContainer";
import { useNotificationStore } from "../../stores/notificationStore";

describe("Toast", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders title and message", () => {
    render(
      <Toast
        notification={{
          id: "n1",
          type: "info",
          title: "Agent started",
          message: "Claude Code is running on feat-auth",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={vi.fn()}
      />
    );
    expect(screen.getByText("Agent started")).toBeDefined();
    expect(screen.getByText("Claude Code is running on feat-auth")).toBeDefined();
  });

  it("renders correct color for error type", () => {
    const { container } = render(
      <Toast
        notification={{
          id: "n2",
          type: "error",
          title: "Agent crashed",
          message: "Process exited with code 1",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={vi.fn()}
      />
    );
    const toastEl = container.firstChild as HTMLElement;
    expect(toastEl.className).toContain("border-red");
  });

  it("renders correct color for success type", () => {
    const { container } = render(
      <Toast
        notification={{
          id: "n3",
          type: "success",
          title: "Done",
          message: "Completed",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={vi.fn()}
      />
    );
    const toastEl = container.firstChild as HTMLElement;
    expect(toastEl.className).toContain("border-green");
  });

  it("calls onDismiss when close button is clicked", () => {
    const onDismiss = vi.fn();
    render(
      <Toast
        notification={{
          id: "n1",
          type: "info",
          title: "Test",
          message: "Test message",
          autoCloseMs: 0,
          createdAt: Date.now(),
        }}
        onDismiss={onDismiss}
      />
    );
    fireEvent.click(screen.getByTitle("Dismiss"));
    expect(onDismiss).toHaveBeenCalledWith("n1");
  });
});

describe("ToastContainer", () => {
  beforeEach(() => {
    cleanup();
    act(() => {
      useNotificationStore.getState().clearAll();
    });
  });

  it("renders no toasts when store is empty", () => {
    const { container } = render(<ToastContainer />);
    expect(container.querySelectorAll("[data-testid='toast']").length).toBe(0);
  });

  it("renders toasts from the notification store", () => {
    act(() => {
      useNotificationStore.getState().addNotification({
        type: "success",
        title: "Workspace created",
        message: "feat-auth ready",
        autoCloseMs: 0,
      });
    });

    render(<ToastContainer />);
    expect(screen.getByText("Workspace created")).toBeDefined();
    expect(screen.getByText("feat-auth ready")).toBeDefined();
  });

  it("respects maxVisible limit", () => {
    act(() => {
      const store = useNotificationStore.getState();
      for (let i = 0; i < 8; i++) {
        store.addNotification({
          type: "info",
          title: `Notification ${i}`,
          message: `Message ${i}`,
          autoCloseMs: 0,
        });
      }
    });

    const { container } = render(<ToastContainer />);
    const toasts = container.querySelectorAll("[data-testid='toast']");
    expect(toasts.length).toBeLessThanOrEqual(5);
  });
});
