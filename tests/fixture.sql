create database example_db;

\c example_db;

--
set password_encryption to md5;
create user example_user_1 with password 'test';
alter user example_user_1 with superuser;

create user example_user_auth_md5 with password 'test';
alter user example_user_auth_md5 with superuser;

create user example_user_jwt with password 'test';
alter user example_user_jwt with superuser;
--
set password_encryption to "scram-sha-256";
create user example_user_2 with password 'test';

-- unix socket.
-- alter system set unix_socket_directories to '/tmp';