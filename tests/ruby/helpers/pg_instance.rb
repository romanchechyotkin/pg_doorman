require 'pg'

class PgInstance
  attr_reader :port
  attr_reader :username
  attr_reader :password
  attr_reader :database_name

  def initialize(port, username, password, database_name)
    @port = port.to_i
    @username = username
    @password = password
    @database_name = database_name
  end

  def with_connection
    conn = PG.connect("postgres://#{@username}:#{@password}@localhost:#{port}/#{database_name}")
    yield conn
  ensure
    conn&.close
  end

  def reset
    drop_connections
    sleep 0.1
  end

  def drop_connections
    username = with_connection { |c| c.async_exec("SELECT current_user")[0]["current_user"] }
    with_connection { |c| c.async_exec("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND usename='#{username}'") }
  end

  def count_connections
    with_connection { |c| c.async_exec("SELECT COUNT(*) as count FROM pg_stat_activity")[0]["count"].to_i }
  end

end