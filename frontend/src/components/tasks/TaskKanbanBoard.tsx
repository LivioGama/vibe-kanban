import { memo } from 'react';
import { Play, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  type DragEndEvent,
  KanbanBoard,
  KanbanCards,
  KanbanHeader,
  KanbanProvider,
} from '@/components/ui/shadcn-io/kanban';
import { TaskCard } from './TaskCard';
import type { TaskStatus, TaskWithAttemptStatus } from 'shared/types';
import { statusBoardColors, statusLabels } from '@/utils/statusLabels';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useTaskMutations } from '@/hooks/useTaskMutations';

export type KanbanColumns = Record<TaskStatus, TaskWithAttemptStatus[]>;

interface TaskKanbanBoardProps {
  columns: KanbanColumns;
  onDragEnd: (event: DragEndEvent) => void;
  onViewTaskDetails: (task: TaskWithAttemptStatus) => void;
  selectedTaskId?: string;
  onCreateTask?: () => void;
  projectId: string;
}

function TaskKanbanBoard({
  columns,
  onDragEnd,
  onViewTaskDetails,
  selectedTaskId,
  onCreateTask,
  projectId,
}: TaskKanbanBoardProps) {
  const { t } = useTranslation('tasks');
  const { startAllTodo } = useTaskMutations(projectId);

  return (
    <KanbanProvider onDragEnd={onDragEnd}>
      {Object.entries(columns).map(([status, tasks]) => {
        const statusKey = status as TaskStatus;
        const isTodo = statusKey === 'todo';

        return (
          <KanbanBoard key={status} id={statusKey}>
            <KanbanHeader
              name={statusLabels[statusKey]}
              color={statusBoardColors[statusKey]}
              onAddTask={onCreateTask}
              extraActions={
                isTodo && tasks.length > 0 ? (
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          className="m-0 p-0 h-4 w-4 text-foreground/50 hover:text-foreground"
                          onClick={() => startAllTodo.mutate(projectId)}
                          disabled={startAllTodo.isPending}
                        >
                          {startAllTodo.isPending ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <Play className="h-4 w-4 fill-current" />
                          )}
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent side="top">
                        {t('actions.startAll')}
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                ) : null
              }
            />
            <KanbanCards>
              {tasks.map((task, index) => (
                <TaskCard
                  key={task.id}
                  task={task}
                  index={index}
                  status={statusKey}
                  onViewDetails={onViewTaskDetails}
                  isOpen={selectedTaskId === task.id}
                  projectId={projectId}
                />
              ))}
            </KanbanCards>
          </KanbanBoard>
        );
      })}
    </KanbanProvider>
  );
}

export default memo(TaskKanbanBoard);
