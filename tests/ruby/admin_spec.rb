# frozen_string_literal: true
require 'uri'
require_relative 'spec_helper'

describe "Admin" do
  let(:processes) { Helpers::PgDoorman.single_instance_setup("example_db", 10) }

  after do
    processes.all_databases.map(&:reset)
    processes.pg_doorman.shutdown
  end

  describe "SHOW ALL" do
      it "can execute all admin queries" do
        admin_conn = PG::connect(processes.pg_doorman.admin_connection_string)
        ["HELP", "CONFIG", "DATABASES", "POOLS", "POOLS_EXTENDED", "CLIENTS",
            "SERVERS", "USERS", "VERSION", "LISTS", "CONNECTIONS", "STATS"].each do |cmd|
           admin_conn.async_exec("SHOW #{cmd}")
        end
        admin_conn.close
      end
  end

  describe "SHOW USERS" do
      it "returns the right users" do
        admin_conn = PG::connect(processes.pg_doorman.admin_connection_string)
        results = admin_conn.async_exec("SHOW USERS")[0]
        admin_conn.close
        expect(results["name"]).to eq("example_user_1")
        expect(results["pool_mode"]).to eq("transaction")
      end
  end

  describe "SHOW CONNECTIONS" do
      it "returns the connection states" do
        admin_conn = PG::connect(processes.pg_doorman.admin_connection_string)
        results = admin_conn.async_exec("SHOW CONNECTIONS")[0]
        admin_conn.close
        expect(results["total"].to_i).to be > 0
      end
  end

end