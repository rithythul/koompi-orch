import { useCallback } from "react";
import {
  DndContext,
  DragOverlay,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

export interface PipelineStep {
  id: string;
  role: string;
  agentType: string;
  handoffType?: "summary" | "full_log" | "diff_only";
}

interface PipelineBuilderProps {
  steps: PipelineStep[];
  onStepsChange: (steps: PipelineStep[]) => void;
}

const ROLES = [
  "architect",
  "implementer",
  "reviewer",
  "tester",
  "shipper",
  "fixer",
];

const AGENT_TYPES = ["claude-code", "codex", "gemini-cli", "aider", "custom"];

const HANDOFF_TYPES: PipelineStep["handoffType"][] = [
  "summary",
  "full_log",
  "diff_only",
];

const ROLE_COLORS: Record<string, { border: string; badge: string; text: string }> = {
  architect: { border: "border-l-purple-400", badge: "bg-purple-400/10 text-purple-400", text: "text-purple-400" },
  implementer: { border: "border-l-blue-400", badge: "bg-blue-400/10 text-blue-400", text: "text-blue-400" },
  reviewer: { border: "border-l-amber-400", badge: "bg-amber-400/10 text-amber-400", text: "text-amber-400" },
  tester: { border: "border-l-emerald-400", badge: "bg-emerald-400/10 text-emerald-400", text: "text-emerald-400" },
  shipper: { border: "border-l-orange-400", badge: "bg-orange-400/10 text-orange-400", text: "text-orange-400" },
  fixer: { border: "border-l-red-400", badge: "bg-red-400/10 text-red-400", text: "text-red-400" },
};

function SortableStep({
  step,
  index,
  onRemove,
  onChange,
  showHandoff,
}: {
  step: PipelineStep;
  index: number;
  onRemove: () => void;
  onChange: (patch: Partial<PipelineStep>) => void;
  showHandoff: boolean;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: step.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
  };

  const colors = ROLE_COLORS[step.role] ?? { border: "border-l-gray-500", badge: "bg-card-bg-hover text-text-ghost", text: "text-text-ghost" };

  return (
    <>
      <div
        ref={setNodeRef}
        style={style}
        className={`card-glass rounded-lg ${colors.border} border-l-[3px] flex items-center gap-3 px-4 py-3 transition-all duration-150`}
      >
        {/* Drag handle */}
        <div
          {...attributes}
          {...listeners}
          className="cursor-grab text-text-ghost hover:text-text-tertiary transition-colors"
        >
          <svg width="10" height="14" viewBox="0 0 10 14" fill="currentColor">
            <circle cx="3" cy="2" r="1.2"/><circle cx="7" cy="2" r="1.2"/>
            <circle cx="3" cy="7" r="1.2"/><circle cx="7" cy="7" r="1.2"/>
            <circle cx="3" cy="12" r="1.2"/><circle cx="7" cy="12" r="1.2"/>
          </svg>
        </div>

        {/* Step number */}
        <span className={`w-6 h-6 rounded-md flex items-center justify-center text-[11px] font-bold font-mono ${colors.badge}`}>
          {index + 1}
        </span>

        {/* Selects */}
        <select
          value={step.role}
          onChange={(e) => onChange({ role: e.target.value })}
          className="bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[12px] font-medium text-text-primary focus:outline-none focus:border-accent transition-colors cursor-pointer"
        >
          {ROLES.map((r) => (
            <option key={r} value={r}>{r}</option>
          ))}
        </select>

        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-text-ghost shrink-0">
          <path d="M6 4L10 8L6 12"/>
        </svg>

        <select
          value={step.agentType}
          onChange={(e) => onChange({ agentType: e.target.value })}
          className="bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[12px] font-mono text-text-secondary focus:outline-none focus:border-accent transition-colors cursor-pointer"
        >
          {AGENT_TYPES.map((a) => (
            <option key={a} value={a}>{a}</option>
          ))}
        </select>

        {/* Remove */}
        <button
          type="button"
          title="Remove step"
          onClick={onRemove}
          className="ml-auto w-6 h-6 rounded-md flex items-center justify-center text-text-ghost hover:text-error hover:bg-error-muted transition-all duration-150 opacity-0 group-hover:opacity-100"
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
            <path d="M4 4L12 12M12 4L4 12"/>
          </svg>
        </button>
      </div>

      {/* Handoff connector */}
      {showHandoff && (
        <div className="flex items-center gap-2 pl-[52px] py-0.5">
          <div className="w-px h-5 bg-border" />
          <select
            value={step.handoffType ?? "summary"}
            onChange={(e) =>
              onChange({
                handoffType: e.target.value as PipelineStep["handoffType"],
              })
            }
            className="bg-card-bg border border-border rounded px-2 py-0.5 text-[10px] font-mono text-text-ghost focus:outline-none focus:border-accent transition-colors cursor-pointer"
          >
            {HANDOFF_TYPES.map((h) => (
              <option key={h} value={h}>{h}</option>
            ))}
          </select>
          <div className="w-px h-5 bg-border" />
        </div>
      )}
    </>
  );
}

export function PipelineBuilder({
  steps,
  onStepsChange,
}: PipelineBuilderProps) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || active.id === over.id) return;

      const oldIndex = steps.findIndex((s) => s.id === active.id);
      const newIndex = steps.findIndex((s) => s.id === over.id);
      if (oldIndex === -1 || newIndex === -1) return;

      onStepsChange(arrayMove(steps, oldIndex, newIndex));
    },
    [steps, onStepsChange]
  );

  const addStep = useCallback(() => {
    const newStep: PipelineStep = {
      id: `step-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      role: "implementer",
      agentType: "claude-code",
      handoffType: "summary",
    };
    onStepsChange([...steps, newStep]);
  }, [steps, onStepsChange]);

  const removeStep = useCallback(
    (id: string) => {
      onStepsChange(steps.filter((s) => s.id !== id));
    },
    [steps, onStepsChange]
  );

  const updateStep = useCallback(
    (id: string, patch: Partial<PipelineStep>) => {
      onStepsChange(
        steps.map((s) => (s.id === id ? { ...s, ...patch } : s))
      );
    },
    [steps, onStepsChange]
  );

  return (
    <div className="flex flex-col gap-1 max-w-2xl">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-[12px] font-semibold text-text-secondary uppercase tracking-wider">Pipeline Steps</h3>
        <span className="text-[10px] font-mono text-text-ghost">
          {steps.length} step{steps.length !== 1 ? "s" : ""}
        </span>
      </div>

      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={steps.map((s) => s.id)}
          strategy={verticalListSortingStrategy}
        >
          <div className="stagger-children">
            {steps.map((step, index) => (
              <SortableStep
                key={step.id}
                step={step}
                index={index}
                onRemove={() => removeStep(step.id)}
                onChange={(patch) => updateStep(step.id, patch)}
                showHandoff={index < steps.length - 1}
              />
            ))}
          </div>
        </SortableContext>
        <DragOverlay />
      </DndContext>

      <button
        type="button"
        onClick={addStep}
        className="mt-3 px-4 py-2.5 text-[12px] font-medium text-text-ghost hover:text-text-secondary border border-dashed border-border hover:border-border-strong rounded-lg transition-all duration-150 flex items-center justify-center gap-2"
      >
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M8 3V13M3 8H13"/>
        </svg>
        Add Step
      </button>
    </div>
  );
}
