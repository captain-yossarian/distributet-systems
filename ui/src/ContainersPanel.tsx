import type { ContainerInfo } from './types'

export default function ContainersPanel({ containers }: { containers: ContainerInfo[] }) {
  return (
    <section className="panel">
      <div className="panel-header">
        <h2>Containers</h2>
        <span className="panel-header-meta">{containers.length} running</span>
      </div>
      <div className="container-list">
        {containers.length === 0 && <p className="empty">no containers found</p>}
        {containers.map((container) => (
          <div className="container-row" key={container.id}>
            <span className={`status-dot${container.state === 'running' ? ' online' : ' offline'}`} />
            <span className="container-name">{container.name}</span>
            <span className="container-image" title={container.image}>
              {container.image}
            </span>
            <span className="container-status">{container.status}</span>
          </div>
        ))}
      </div>
    </section>
  )
}
