export default function Header({
  healthyCount,
  totalCount,
  onAddSecondary,
  spawning,
}: {
  healthyCount: number
  totalCount: number
  onAddSecondary: () => Promise<void>
  spawning: boolean
}) {
  const allHealthy = healthyCount === totalCount
  return (
    <header className="app-header">
      <div className="app-header-title">
        <h1>Replication Console</h1>
        <span className="app-header-subtitle">master / secondary</span>
      </div>
      <div className="app-header-actions">
        <span className="health-pill">
          <span className={`status-dot${allHealthy ? ' online' : ' offline'}`} />
          {allHealthy
            ? `${healthyCount} node${healthyCount === 1 ? '' : 's'} healthy`
            : `${healthyCount} of ${totalCount} nodes healthy`}
        </span>
        <button
          type="button"
          className="add-secondary-button"
          onClick={onAddSecondary}
          disabled={spawning}
        >
          {spawning ? 'Starting…' : '+ Add secondary'}
        </button>
      </div>
    </header>
  )
}
