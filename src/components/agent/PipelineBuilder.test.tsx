import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { PipelineBuilder } from "./PipelineBuilder";

vi.mock("@dnd-kit/core", () => ({
  DndContext: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dnd-context">{children}</div>
  ),
  DragOverlay: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  closestCenter: vi.fn(),
  PointerSensor: vi.fn(),
  useSensor: vi.fn(),
  useSensors: vi.fn().mockReturnValue([]),
}));

vi.mock("@dnd-kit/sortable", () => ({
  SortableContext: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  verticalListSortingStrategy: "vertical",
  useSortable: vi.fn().mockReturnValue({
    attributes: {},
    listeners: {},
    setNodeRef: vi.fn(),
    transform: null,
    transition: null,
    isDragging: false,
  }),
  arrayMove: vi.fn((arr: unknown[], from: number, to: number) => {
    const result = [...arr];
    const [item] = result.splice(from, 1);
    result.splice(to, 0, item);
    return result;
  }),
}));

vi.mock("@dnd-kit/utilities", () => ({
  CSS: { Transform: { toString: () => "" } },
}));

describe("PipelineBuilder", () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders pipeline steps with step numbers", () => {
    render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "claude-code" },
          { id: "s2", role: "implementer", agentType: "claude-code" },
          { id: "s3", role: "reviewer", agentType: "claude-code" },
        ]}
        onStepsChange={vi.fn()}
      />
    );
    expect(screen.getByText("1")).toBeDefined();
    expect(screen.getByText("2")).toBeDefined();
    expect(screen.getByText("3")).toBeDefined();
    expect(screen.getByText("3 steps")).toBeDefined();
  });

  it("renders add step button", () => {
    render(
      <PipelineBuilder steps={[]} onStepsChange={vi.fn()} />
    );
    expect(screen.getByText("+ Add Step")).toBeDefined();
  });

  it("calls onStepsChange when a step is removed", () => {
    const onChange = vi.fn();
    render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "claude-code" },
          { id: "s2", role: "implementer", agentType: "claude-code" },
        ]}
        onStepsChange={onChange}
      />
    );
    const removeButtons = screen.getAllByTitle("Remove step");
    fireEvent.click(removeButtons[0]);
    expect(onChange).toHaveBeenCalledWith([
      { id: "s2", role: "implementer", agentType: "claude-code" },
    ]);
  });

  it("calls onStepsChange when add step is clicked", () => {
    const onChange = vi.fn();
    render(
      <PipelineBuilder steps={[]} onStepsChange={onChange} />
    );
    fireEvent.click(screen.getByText("+ Add Step"));
    expect(onChange).toHaveBeenCalledWith([
      expect.objectContaining({
        role: "implementer",
        agentType: "claude-code",
      }),
    ]);
  });

  it("renders role selects with correct values", () => {
    const { container } = render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "codex" },
        ]}
        onStepsChange={vi.fn()}
      />
    );
    const selects = container.querySelectorAll("select");
    // First select is role, second is agent type
    expect((selects[0] as HTMLSelectElement).value).toBe("architect");
    expect((selects[1] as HTMLSelectElement).value).toBe("codex");
  });

  it("renders handoff type selector between steps", () => {
    const { container } = render(
      <PipelineBuilder
        steps={[
          { id: "s1", role: "architect", agentType: "claude-code", handoffType: "summary" },
          { id: "s2", role: "implementer", agentType: "claude-code" },
        ]}
        onStepsChange={vi.fn()}
      />
    );
    // There should be a handoff select between steps (3rd select after the role and agent selects of step 1)
    const selects = container.querySelectorAll("select");
    // s1: role, agent, handoff; s2: role, agent
    expect(selects.length).toBe(5);
    expect((selects[2] as HTMLSelectElement).value).toBe("summary");
  });
});
