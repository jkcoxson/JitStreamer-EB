create table devices (
  ip varchar(15) primary key,
  udid varchar(40) not null,
  last_used datetime not null
);


create table downloads (
  code varchar(40) primary key,
  contents varchar(255) not null
);

create table launch_queue (
  udid varchar(40) not null,
  ip varchar(32) not null,
  bundle_id varchar(255) not null,
  status int not null, -- 0: pending, 2: error
  error varchar(255),
  ordinal integer primary key
);
