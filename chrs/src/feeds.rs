use futures::{pin_mut, TryStreamExt};
use chris::ChrisClient;

pub(crate) async fn list_feeds(chris: &ChrisClient, limit: u32) -> anyhow::Result<()> {
    if limit == 0 {
        return Ok(())
    }
    let search = chris.search_feeds();
    pin_mut!(search);

    let mut count = 0;
    while let Some(feed) = search.try_next().await? {
        println!("{:<72} {}/feed_{}", feed.name, feed.creator_username, feed.id);
        count += 1;
        if count >= limit {
            break;
        }
    }
    Ok(())
}
