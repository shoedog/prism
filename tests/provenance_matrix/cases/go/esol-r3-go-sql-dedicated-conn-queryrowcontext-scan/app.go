package fixture

import (
  "context"
  "database/sql"
  "database/sql/driver"
)

func label(ctx context.Context, connector driver.Connector) (string, error) {
  pool := sql.OpenDB(connector)
  defer pool.Close()
  conn, err := pool.Conn(ctx)
  if err != nil { return "", err }
  defer conn.Close()
  var title string
  scanErr := conn.QueryRowContext(ctx, "select label from catalog limit 1").Scan(&title)
  return title, scanErr
}
