import {
  DndContext,
  DragOverlay,
  closestCorners,
  PointerSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type Workspace,
  type WorkspaceStatus,
  useWorkspaceStore,
} from "../../stores/workspaceStore";
import { WorkspaceCard } from "./WorkspaceCard";

const COLUMNS: { status: WorkspaceStatus; label: string; color: string }[] = [
  { status: "backlog", label: "Backlog", color: "border-gray-600" },
  { status: "active", label: "Active", color: "border-blue-600" },
  { status: "review", label: "Review", color: "border-yellow-600" },
  { status: "done", label: "Done", color: "border-green-600" },
  { status: "failed", label: "Failed", color: "border-red-600" },
];

/** A sortable workspace card inside a kanban column */
function SortableWorkspaceCard({ workspace }: { workspace: Workspace }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: workspace.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <WorkspaceCard workspace={workspace} />
    </div>
  );
}

/** A single kanban column for a workspace status */
function KanbanColumn({
  status,
  label,
  color,
  workspaces,
}: {
  status: WorkspaceStatus;
  label: string;
  color: string;
  workspaces: Workspace[];
}) {
  return (
    <div
      className={`flex-1 min-w-[180px] max-w-[280px] border-t-2 ${color} bg-gray-900/50 rounded-lg p-2`}
      data-column-status={status}
    >
      <div className="flex items-center justify-between mb-2 px-1">
        <h3 className="text-xs font-semibold uppercase text-gray-400">
          {label}
        </h3>
        <span className="text-xs text-gray-600">{workspaces.length}</span>
      </div>
      <SortableContext
        items={workspaces.map((w) => w.id)}
        strategy={verticalListSortingStrategy}
      >
        <div className="flex flex-col gap-1 min-h-[60px]">
          {workspaces.map((workspace) => (
            <SortableWorkspaceCard key={workspace.id} workspace={workspace} />
          ))}
        </div>
      </SortableContext>
    </div>
  );
}

export function KanbanBoard() {
  const workspacesByStatus = useWorkspaceStore((s) => s.workspacesByStatus);
  const updateWorkspace = useWorkspaceStore((s) => s.updateWorkspace);
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const [activeId, setActiveId] = useState<string | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 5 },
    })
  );

  const grouped = workspacesByStatus();
  const activeWorkspace = activeId
    ? workspaces.find((w) => w.id === activeId)
    : null;

  const handleDragStart = useCallback((event: DragStartEvent) => {
    setActiveId(String(event.active.id));
  }, []);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      setActiveId(null);
      const { active, over } = event;
      if (!over) return;

      const draggedId = String(active.id);
      const overId = String(over.id);

      const draggedWorkspace = workspaces.find((w) => w.id === draggedId);
      if (!draggedWorkspace) return;

      const overWorkspace = workspaces.find((w) => w.id === overId);
      let targetStatus: WorkspaceStatus | undefined;

      if (overWorkspace) {
        targetStatus = overWorkspace.status;
      } else {
        const overElement = document.querySelector(
          `[data-column-status="${overId}"]`
        );
        if (overElement) {
          targetStatus = overId as WorkspaceStatus;
        }
      }

      if (targetStatus && targetStatus !== draggedWorkspace.status) {
        updateWorkspace(draggedId, { status: targetStatus });
        invoke("update_workspace_status", {
          workspaceId: draggedId,
          status: targetStatus,
        }).catch((err: unknown) => {
          console.error("Failed to update workspace status:", err);
          updateWorkspace(draggedId, { status: draggedWorkspace.status });
        });
      }
    },
    [workspaces, updateWorkspace]
  );

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <div className="flex gap-3 overflow-x-auto p-2">
        {COLUMNS.map(({ status, label, color }) => (
          <KanbanColumn
            key={status}
            status={status}
            label={label}
            color={color}
            workspaces={grouped[status]}
          />
        ))}
      </div>
      <DragOverlay>
        {activeWorkspace ? (
          <div className="opacity-80">
            <WorkspaceCard workspace={activeWorkspace} />
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}
