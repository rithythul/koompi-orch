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

const ROLE_COLORS: Record<string, string> = {
  architect: "border-l-purple-500",
  implementer: "border-l-blue-500",
  reviewer: "border-l-yellow-500",
  tester: "border-l-green-500",
  shipper: "border-l-orange-500",
  fixer: "border-l-red-500",
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

  return (
    <>
      <div
        ref={setNodeRef}
        style={style}
        className={`flex items-center gap-3 px-4 py-3 bg-gray-800/50 border border-gray-700 ${
          ROLE_COLORS[step.role] ?? "border-l-gray-500"
        } border-l-4 rounded-lg`}
      >
        <div
          {...attributes}
          {...listeners}
          className="cursor-grab text-gray-600 hover:text-gray-400"
        >
          <span className="text-xs select-none">::</span>
        </div>

        <span className="w-6 h-6 rounded-full bg-gray-700 flex items-center justify-center text-xs font-bold text-gray-300">
          {index + 1}
        </span>

        <select
          value={step.role}
          onChange={(e) => onChange({ role: e.target.value })}
          className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
        >
          {ROLES.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>

        <select
          value={step.agentType}
          onChange={(e) => onChange({ agentType: e.target.value })}
          className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
        >
          {AGENT_TYPES.map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>

        <button
          type="button"
          title="Remove step"
          onClick={onRemove}
          className="ml-auto text-gray-600 hover:text-red-400 text-sm"
        >
          &times;
        </button>
      </div>

      {showHandoff && (
        <div className="flex items-center gap-2 pl-12 py-1">
          <div className="w-px h-4 bg-gray-700" />
          <select
            value={step.handoffType ?? "summary"}
            onChange={(e) =>
              onChange({
                handoffType: e.target.value as PipelineStep["handoffType"],
              })
            }
            className="bg-gray-900 border border-gray-700 rounded px-2 py-0.5 text-[10px] text-gray-400 focus:outline-none focus:border-blue-500"
          >
            {HANDOFF_TYPES.map((h) => (
              <option key={h} value={h}>
                {h}
              </option>
            ))}
          </select>
          <div className="w-px h-4 bg-gray-700" />
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
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-sm font-medium text-gray-300">Pipeline Steps</h3>
        <span className="text-xs text-gray-500">
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
        </SortableContext>
        <DragOverlay />
      </DndContext>

      <button
        type="button"
        onClick={addStep}
        className="mt-2 px-4 py-2 text-sm text-gray-400 hover:text-gray-200 border border-dashed border-gray-700 hover:border-gray-500 rounded-lg transition-colors"
      >
        + Add Step
      </button>
    </div>
  );
}
