import { parseStringArray, tacticName, techniqueName } from '@/shared/lib/mitre'

interface MitreBadgesProps {
  tactics?: string[] | string | null
  techniques?: string[] | string | null
  compact?: boolean
}

export function MitreBadges({ tactics, techniques, compact = false }: MitreBadgesProps) {
  const tacticIds = parseStringArray(tactics)
  const techniqueIds = parseStringArray(techniques)

  if (tacticIds.length === 0 && techniqueIds.length === 0) {
    return null
  }

  const size = compact ? 'text-xs px-1.5 py-0.5' : 'text-xs px-2 py-1'

  return (
    <div className="flex flex-wrap gap-1.5 items-center">
      {tacticIds.map((t) => (
        <span
          key={t}
          title={`Tactic: ${tacticName(t)}`}
          className={`rounded-full bg-blue-50 text-blue-700 font-mono ${size}`}
        >
          {tacticName(t)}
        </span>
      ))}
      {techniqueIds.map((t) => (
        <span
          key={t}
          title={`Technique: ${techniqueName(t)}`}
          className={`rounded-full bg-indigo-50 text-indigo-700 font-mono ${size}`}
        >
          {t}
        </span>
      ))}
    </div>
  )
}
