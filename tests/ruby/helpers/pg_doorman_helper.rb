require 'json'
require 'ostruct'
require_relative 'pg_doorman_process'
require_relative 'pg_instance'
require_relative 'pg_socket'

class ::Hash
    def deep_merge(second)
        merger = proc { |key, v1, v2| Hash === v1 && Hash === v2 ? v1.merge(v2, &merger) : v2 }
        self.merge(second, &merger)
    end
end

module Helpers
  module PgDoorman

    def self.single_instance_setup(pool_name, pool_size, pool_mode="transaction", log_level="trace")
      user = {
        "password" => "md58a67a0c805a5ee0384ea28e0dea557b6", # test
        "pool_size" => pool_size,
        "username" => "example_user_1",
        "pool_mode" => pool_mode,
      }

      pg_doorman = PgDoormanProcess.new(log_level)
      pg_doorman_cfg = pg_doorman.current_config

      primary  = PgInstance.new(5432, user["username"], "test", "example_db")

      # Main proxy configs
      pg_doorman_cfg["pools"] = {
        "#{pool_name}" => {
          "server_host" => "localhost",
          "server_port" => primary.port.to_i,
          "users" => { "0" => user }
        }
      }
      pg_doorman_cfg["general"]["port"] = pg_doorman.port
      pg_doorman.update_config(pg_doorman_cfg)
      pg_doorman.start
      pg_doorman.wait_until_ready

      OpenStruct.new.tap do |struct|
        struct.pg_doorman = pg_doorman
        struct.primary = primary
        struct.all_databases = [primary]
      end
    end

  end
end