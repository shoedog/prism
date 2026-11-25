async fn load(pool: &sqlx::SqlitePool) -> Result<i64, sqlx::Error> {
    let builder = sqlx::query_scalar::<_, i64>("select 1");
    let payload = builder.fetch_one(pool).await?;
    Ok(payload)
}

fn prepare() {
    let builder = sqlx::query_scalar::<sqlx::Sqlite, i64>("select 1");
    drop(builder);
}
