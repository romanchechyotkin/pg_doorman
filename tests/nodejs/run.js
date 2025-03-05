const { Client } = require('pg');

const client = new Client({
	user: 'example_user_1',
	password: 'test',
	host: '127.0.0.1',
	port: '6433',
	database: 'example_db',
});

// Connect to the database
client
	.connect()
	.then(() => {
		console.log('Connected to PostgreSQL database');

		// Execute SQL queries here

		client.query('drop table if exists node_users');
		client.query('create table node_users (id serial primary key, name text)');
		client.query('insert into node_users(name) values ($1)', ['Dima']);
		client.query('select * from node_users where name = $1', ['Dima']);
		client.query('select * from node_users', (err, _) => { if (err) { throw Error (err); }
			client
				.end()
				.then(() => {
					console.log('Connection to PostgreSQL closed');
				})
				.catch((err) => {
					throw Error(err);
				});
		});
	})
	.catch((err) => {
		console.error('Error connecting to PostgreSQL database', err);
	});