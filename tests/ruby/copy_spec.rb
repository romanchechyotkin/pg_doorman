# frozen_string_literal: true
require_relative 'spec_helper'


describe "COPY Handling" do
  let(:processes) { Helpers::PgDoorman.single_instance_setup("example_db", 5) }
  before do
    new_configs = processes.pg_doorman.current_config

    # Allow connections in the pool to expire faster
    new_configs["general"]["idle_timeout"] = 5
    processes.pg_doorman.update_config(new_configs)
    # We need to kill the old process that was using the default configs
    processes.pg_doorman.stop
    processes.pg_doorman.start
    processes.pg_doorman.wait_until_ready
  end

  before do
    processes.all_databases.first.with_connection do |conn|
      conn.async_exec "CREATE TABLE copy_test_table (a TEXT,b TEXT,c TEXT,d TEXT)"
    end
  end

  after do
    processes.all_databases.first.with_connection do |conn|
      conn.async_exec "DROP TABLE copy_test_table;"
    end
  end

  after do
    processes.all_databases.map(&:reset)
    processes.pg_doorman.shutdown
  end

  describe "COPY FROM" do
    context "within transaction" do
      it "finishes within alloted time" do
        conn = PG.connect(processes.pg_doorman.connection_string("example_db", "example_user_1", "test"))
        Timeout.timeout(3) do
          conn.async_exec("BEGIN")
          conn.copy_data "COPY copy_test_table FROM STDIN CSV" do
            sleep 0.5
            conn.put_copy_data "some,data,to,copy\n"
            conn.put_copy_data "more,data,to,copy\n"
          end
          conn.async_exec("COMMIT")
        end

        res = conn.async_exec("SELECT * FROM copy_test_table").to_a
        expect(res).to eq([
          {"a"=>"some", "b"=>"data", "c"=>"to", "d"=>"copy"},
          {"a"=>"more", "b"=>"data", "c"=>"to", "d"=>"copy"}
        ])
      end
    end

    context "outside transaction" do
      it "finishes within alloted time" do
        conn = PG.connect(processes.pg_doorman.connection_string("example_db", "example_user_1", "test"))
        Timeout.timeout(3) do
          conn.copy_data "COPY copy_test_table FROM STDIN CSV" do
            sleep 0.5
            conn.put_copy_data "some,data,to,copy\n"
            conn.put_copy_data "more,data,to,copy\n"
          end
        end

        res = conn.async_exec("SELECT * FROM copy_test_table").to_a
        expect(res).to eq([
          {"a"=>"some", "b"=>"data", "c"=>"to", "d"=>"copy"},
          {"a"=>"more", "b"=>"data", "c"=>"to", "d"=>"copy"}
        ])
      end
    end
  end

  describe "COPY TO" do
    before do
      conn = PG.connect(processes.pg_doorman.connection_string("example_db", "example_user_1", "test"))
      conn.async_exec("BEGIN")
      conn.copy_data "COPY copy_test_table FROM STDIN CSV" do
        conn.put_copy_data "some,data,to,copy\n"
        conn.put_copy_data "more,data,to,copy\n"
      end
      conn.async_exec("COMMIT")
      conn.close
    end

    it "works" do
      res = []
      conn = PG.connect(processes.pg_doorman.connection_string("example_db", "example_user_1", "test"))
      conn.copy_data "COPY copy_test_table TO STDOUT CSV" do
        while row=conn.get_copy_data
          res << row
        end
      end
      expect(res).to eq(["some,data,to,copy\n", "more,data,to,copy\n"])
    end
  end

end