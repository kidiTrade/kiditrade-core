create table contracts (
    id serial primary key,
    symbol varchar not null,
    exchange varchar not null,
    currency varchar not null
);

create table bars (
    contractId integer references contracts(id),
    timestamp timestamptz not null,
    duration interval not null,
    open numeric not null,
    high numeric not null,
    low numeric not null,
    close numeric not null,
    vwap numeric not null,
    volume numeric not null,
    trades numeric not null,
    primary key (contractId, timestamp, duration)
);

insert into contracts (symbol, exchange, currency)
values
    ('ROKU', 'SMART', 'USD'),
    ('TSLA', 'SMART', 'USD'),
    ('NVDA', 'SMART', 'USD');
