import { useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useWorkspace } from '@/hooks/useWorkspace'
import { toast } from 'sonner'
import { useTranslation } from '@/i18n/react-i18next-compat'
import { IconFile, IconFolderOpen } from '@tabler/icons-react'
import { getServiceHub } from '@/hooks/useServiceHub'

interface AddFileToWorkspaceDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  workspaceId: string
}

export const AddFileToWorkspaceDialog = ({
  open,
  onOpenChange,
  workspaceId,
}: AddFileToWorkspaceDialogProps) => {
  const { t } = useTranslation()
  const { addFileToWorkspace } = useWorkspace()
  const [selectedFilePath, setSelectedFilePath] = useState<string>('')
  const [isAdding, setIsAdding] = useState(false)

  const handleSelectFile = async () => {
    try {
      // Use service hub dialog to select a file
      const selected = await getServiceHub().dialog().open({
        multiple: false,
        directory: false,
      })

      if (selected && typeof selected === 'string') {
        setSelectedFilePath(selected)
      } else if (Array.isArray(selected) && selected.length > 0) {
        setSelectedFilePath(selected[0])
      }
    } catch (error) {
      console.error('Failed to select file:', error)
      toast.error(t('workspace.error.selectFileFailed'))
    }
  }

  const handleAddFile = async () => {
    if (!selectedFilePath) {
      toast.error(t('workspace.error.noFileSelected'))
      return
    }

    setIsAdding(true)
    try {
      // Convert to file:// URI if needed
      const fileUri = selectedFilePath.startsWith('file://')
        ? selectedFilePath
        : `file://${selectedFilePath}`

      await addFileToWorkspace(workspaceId, fileUri)
      toast.success(t('workspace.success.fileAdded'))
      onOpenChange(false)
      setSelectedFilePath('')
    } catch (error) {
      console.error('Failed to add file:', error)
      toast.error(t('workspace.error.addFileFailed'))
    } finally {
      setIsAdding(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('workspace.dialog.addFile.title')}</DialogTitle>
          <DialogDescription>
            {t('workspace.dialog.addFile.description')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="flex items-center gap-2">
            <Button
              variant="default"
              onClick={handleSelectFile}
              className="w-full justify-start"
            >
              <IconFolderOpen size={16} className="mr-2" />
              {selectedFilePath
                ? t('workspace.dialog.addFile.changeFile')
                : t('workspace.dialog.addFile.selectFile')}
            </Button>
          </div>

          {selectedFilePath && (
            <div className="flex items-center gap-2 p-3 bg-muted rounded-md">
              <IconFile size={16} className="text-muted-foreground shrink-0" />
              <span className="text-sm truncate flex-1" title={selectedFilePath}>
                {selectedFilePath}
              </span>
            </div>
          )}

          <p className="text-sm text-muted-foreground">
            {t('workspace.dialog.addFile.note')}
          </p>
        </div>

        <DialogFooter>
          <Button variant="default" onClick={() => onOpenChange(false)}>
            {t('common:cancel')}
          </Button>
          <Button
            onClick={handleAddFile}
            disabled={!selectedFilePath || isAdding}
          >
            {isAdding ? t('common:adding') : t('common:add')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
