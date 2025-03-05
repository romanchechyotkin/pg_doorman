# frozen_string_literal: true
require 'pg'
require 'active_record'

# Uncomment these two to see all queries.
# ActiveRecord.verbose_query_logs = true
ActiveRecord::Base.logger = Logger.new(STDOUT)

ActiveRecord::Base.establish_connection(
  adapter: 'postgresql',
  host: 'localhost',
  port: 6433,
  username: 'example_user_1',
  password: 'test',
  database: 'example_db',
  application_name: 'testing_pg_doorman',
  prepared_statements: true,
  advisory_locks: false
)

class TestSafeTable < ActiveRecord::Base
  self.table_name = 'test_safe_table'
end

class ShouldNeverHappenException < RuntimeError
end

class CreateSafeShardedTable < ActiveRecord::Migration[6.0]
  # Disable transasctions or things will fly out of order!
  disable_ddl_transaction!

  def up
      connection.execute <<-SQL
        CREATE TABLE test_safe_table (
          id BIGINT PRIMARY KEY,
          name VARCHAR,
          description TEXT
        );
      SQL
  end

  def down
      connection.execute 'DROP TABLE test_safe_table CASCADE'
  end
end

CreateSafeShardedTable.migrate(:up)

10.times do |x|
    TestSafeTable.create(id: x, name: "something_special_#{x.to_i}", description: "It's a surprise!")
end

10.times do |x|
    raise "This was expected to be true" unless TestSafeTable.find_by_id(x).name == "something_special_#{x.to_i}"
end

TestSafeTable.connection.execute "select repeat('1', 40000000)"

CreateSafeShardedTable.migrate(:down)