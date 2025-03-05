# frozen_string_literal: true
require_relative 'spec_helper'


describe "Portocol handling" do
  let(:processes) { Helpers::PgDoorman.single_instance_setup("example_db", 5, "session") }
  let(:sequence) { [] }
  let(:pg_doorman_socket) { PostgresSocket.new('localhost', processes.pg_doorman.port) }
  let(:pgdb_socket) { PostgresSocket.new('localhost', processes.all_databases.first.port, false) }

  after do
    pgdb_socket.close
    pg_doorman_socket.close
    processes.all_databases.map(&:reset)

    # print("logs: #{processes.pg_doorman.log}\n")
    processes.pg_doorman.shutdown
  end

  def run_comparison(sequence, socket_a, socket_b)
    sequence.each do |msg, *args|
      socket_a.send(msg, *args)
      socket_b.send(msg, *args)

      compare_messages(
        socket_a.read_from_server,
        socket_b.read_from_server
      )
    end
  end

  def compare_messages(msg_arr0, msg_arr1)
    if msg_arr0.count != msg_arr1.count
      error_output = []

      error_output << "#{msg_arr0.count} : #{msg_arr1.count}"
      error_output << "PgDoorman Messages"
      error_output += msg_arr0.map { |message| "\t#{message[:code]} - #{message[:bytes].map(&:chr).join(" ")}" }
      error_output << "PgServer Messages"
      error_output += msg_arr1.map { |message| "\t#{message[:code]} - #{message[:bytes].map(&:chr).join(" ")}" }
      error_desc = error_output.join("\n")
      raise StandardError, "Message count mismatch #{error_desc}"
    end

    (0..msg_arr0.count - 1).all? do |i|
      msg0 = msg_arr0[i]
      msg1 = msg_arr1[i]

      result = [
        msg0[:code] == msg1[:code],
        msg0[:len] == msg1[:len],
        msg0[:bytes] == msg1[:bytes],
      ].all?

      next result if result

      if result == false
        error_string = []
        if msg0[:code] != msg1[:code]
          error_string << "code #{msg0[:code]} != #{msg1[:code]}"
        end
        if msg0[:len] != msg1[:len]
          error_string << "len #{msg0[:len]} != #{msg1[:len]}"
        end
        if msg0[:bytes] != msg1[:bytes]
          error_string << "bytes #{msg0[:bytes]} != #{msg1[:bytes]}"
        end
        err = error_string.join("\n")

        raise StandardError, "Message mismatch #{err}"
      end
    end
  end

  RSpec.shared_examples "at parity with database" do
    before do
      pg_doorman_socket.send_startup_message("example_user_1", "example_db", "test")
      pgdb_socket.send_startup_message("example_user_1", "example_db", "test")
    end

    it "works" do
      run_comparison(sequence, pg_doorman_socket, pgdb_socket)
    end
  end

  context "Cancel Query" do
    let(:sequence) {
      [
        [:send_query_message, "SELECT pg_sleep(5)"],
        [:cancel_query]
      ]
    }

    it_behaves_like "at parity with database"
  end

  context "Flush message" do
    let(:sequence) {
      [
        [:send_parse_message, "SELECT 1"],
        [:send_flush_message]
      ]
    }

    it_behaves_like "at parity with database"
  end

  context "Simple message" do
    let(:sequence) {
      [[:send_query_message, "SELECT 1"]]
    }

    it_behaves_like "at parity with database"
  end

  context "Health check" do
    let(:sequence) {
      [[:send_query_message, ";"]]
    }

    it_behaves_like "at parity with database"
  end

  context "Double extended protocol" do
    let(:sequence) {
      [
        [:send_parse_message, "SELECT 1"],
        [:latency, 0.5],
        [:send_bind_message],
        [:send_describe_message, "P"],
        [:send_execute_message],
        [:send_parse_message, "SELECT 1"],
        [:latency, 0.5],
        [:send_bind_message],
        [:send_describe_message, "P"],
        [:send_execute_message],
        [:send_sync_message],
      ]
    }

    it_behaves_like "at parity with database"
  end

  context "Extended protocol" do
    let(:sequence) {
      [
        [:send_parse_message, "SELECT 1"],
        [:send_bind_message],
        [:send_describe_message, "P"],
        [:send_execute_message],
        [:send_sync_message],
      ]
    }

    it_behaves_like "at parity with database"
  end
end