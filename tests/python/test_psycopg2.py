import psycopg2
from psycopg2.extras import NamedTupleCursor
import datetime


conn_args = dict(
    host="localhost",
    user="example_user_1",
    password="test",
    dbname="example_db",
    port=6433,
    cursor_factory=NamedTupleCursor,
    sslmode="disable")
conn = psycopg2.connect(**conn_args)
cur = conn.cursor()

cur.execute("drop table if exists users_python;")
cur.execute('''
        create table users_python(
            id serial primary key,
            name text,
            dob date
        )
''')
cur.execute("insert into users_python(name, dob) values(%s, %s)", ('Dima', datetime.date(1983, 12, 12)))
#conn.commit()
cur.close()
conn.close()