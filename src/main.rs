use redis::Redis;

mod executor;
mod redis;

fn main() {
    let mut redis_server = Redis::default();

    redis_server.listen().unwrap();
}
