import { ChevronDown, ChevronUp, Route } from 'lucide-react'
import { useState, memo } from 'react'
import { useTranslation } from '@/i18n/react-i18next-compat'

interface RoutingDecision {
  modelId: string
  providerId: string
  confidence: number
  reasoning: string
}

interface Props {
  routingDecision: RoutingDecision
}

/**
 * RoutingReasonBlock displays the reasoning behind the model routing decision
 * in an expandable/collapsible block below the assistant's response.
 */
const RoutingReasonBlock = memo(({ routingDecision }: Props) => {
  const [isExpanded, setIsExpanded] = useState(false)
  const { t } = useTranslation()

  const handleClick = () => {
    setIsExpanded(!isExpanded)
  }

  // Don't render if there's no meaningful reasoning
  if (!routingDecision?.reasoning || routingDecision.reasoning === 'no reason') {
    return null
  }

  return (
    <div
      className="mx-auto w-full cursor-pointer break-words mt-2"
      onClick={handleClick}
    >
      <div className="rounded-lg bg-main-view-fg/4 border border-dashed border-main-view-fg/10 p-2">
        <div className="flex items-center gap-3">
          <Route className="size-4 text-main-view-fg/60" />
          <button className="flex items-center gap-2 focus:outline-none">
            {isExpanded ? (
              <ChevronUp className="size-4 text-main-view-fg/60" />
            ) : (
              <ChevronDown className="size-4 text-main-view-fg/60" />
            )}
            <span className="font-medium text-main-view-fg/60 text-sm">
              {t('routingReason')}
            </span>
          </button>
        </div>

        {isExpanded && (
          <div className="mt-2 pl-6 pr-4 text-main-view-fg/60 text-sm">
            <div className="space-y-1">
              <div>
                <span className="font-medium">{t('model')}:</span>{' '}
                <span className="text-main-view-fg">{routingDecision.modelId}</span>
              </div>
              <div>
                <span className="font-medium">{t('confidence')}:</span>{' '}
                <span className="text-main-view-fg">
                  {(routingDecision.confidence * 100).toFixed(0)}%
                </span>
              </div>
              <div>
                <span className="font-medium">{t('reason')}:</span>{' '}
                <span className="text-main-view-fg">{routingDecision.reasoning}</span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
})

RoutingReasonBlock.displayName = 'RoutingReasonBlock'

export default RoutingReasonBlock
