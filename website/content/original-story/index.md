---
---

## The Story of Getting Farted On

**While this story is based on real events some facts are actually made up**.

*Getting Farted On* 💩 is a game I made for [Ludum Dare 51](https://ldjam.com/events/ludum-dare/51/getting-farted-on) game jam in 48 hours.

[🎮 Play the game on itch.io](https://kuviman.itch.io/getting-farted-on).

[Source code is available on GitHub](https://github.com/kuviman/getting-farted-on).

*TLDR: fart is good humor*.

# About LudumDare

![ld51](images/ld51.png)

[Ludum Dare](https://ludumdare.com) is one of if not the biggest game jams.
It happens twice a year (although seems like the plan is to start doing it three times a year from now on).

There is multiple ways to participate in it:

- Compo - you have to do everything from scratch by yourself in 48 hours
- Jam - you are allowed to have teammates and use premade assets, and you also have more time - 72 hours
- Extra - new thing introduced in one of recent jams, where you can make a game in 3 weeks without any limitation

As usual, I went for compo.

## Voting for Theme

Before the jam starts the theme needs to be determined.
First people suggest their themes, then voting happens.
I never actually participated in this part but this time I decided to do the voting.

![my final votes](images/final-votes.png)

Now, **Ascend** was clearly the best choice out of all the options in the final voting round, so I voted for it and nothing else. I tried to think a bit about all the other themes in case they win too. Except **Every 10 seconds**, as it was clearly the worst one. I just skipped it and hoped for it not to win.

While we were waiting for the theme announcement, I did some practicing creating a [small battle royale simulator](https://github.com/kuviman/ttv) thing that I was planning to use during the Play+Rate phase on stream.

To test it, I ran a battle with all the themes fighting each other. And **Ascend** has won! First try! What a coincidence!

![voting battle](images/voting-battle.gif)

Speaking about coincidences, there is no such thing in the actual voting results. Obviously all people want is to suffer and make other people suffer.

**EVERY 10 SECONDS** won.

![mike-tweet](images/mike-tweet.png)

### Coming up with idea

Ok, so people chose suffering. Well then, suffering it is.
The decision was made.

I like rage games like Getting Over It, Jump King or Pogostuck and wanted to create one for a while.
So this is the basic idea here.
We make people suffer for choosing this theme by making the hardest jam game ever.

This is particularly good because it aligns with the best theme that could have been chosen - **Ascend**. We need to go up but it should require skill and patience. Otherwise you are falling down. Hopefully to the very beginning 😈.

Now, what about the **Every 10 seconds** part? What could possibly happen every 10 seconds in such type of a game? What could bring even more suffering to the players 🤔?

Well, a game about farting it is! Before you judge me I did check with a friend that it is appropriate to make a game like this

![inappropriate](images/inappropriate.png)

Yea, we make it an MMO too so that people are getting farted on each other.

After some time of brainstorming I came up with the story for the game:

> \- dude this burger was delicious im gona have second one
>
> \- dude you already had 10 “seconds” are you sure
>
> \- dude you right brb i need to poo real quick
>
> Oh no the toilet is out of order
>
> And you really need it very badly, you are already farting every 10 seconds
>
> Looks like your only hope is that shiny one at the top
>
> You are really trying to hold it so you cant even walk like a human. All you can do is roll
>
> Will you be able to make the ascend to stop this every 10 seconds nonsense
>
> Fart your way to the top as soon as possible

So we use fart mechanic to jump every 10 seconds automatically.
Rolling is there so that its hard to get both good position and direction of a jump so the game is more challenging. To add more to it, other than the auto farting we are also going to be able to force a fart every now and then so that we can do double jumps. With good timing you can **ascend** quicker 😉.

![concept](images/concept.png)

### Tools

Now that we have an idea its time to get the tools ready. Here's what I'm using this time, same as usual:

- [Rust programming language](https://www.rust-lang.org/) 🦀
- [My custom engine](https://github.com/kuviman/geng)
- [VS Code](https://code.visualstudio.com/)
- [Paint.Net](https://getpaint.net/) 🖼️
- [Audacity](https://www.audacityteam.org/) 🎶
- Microphone 🎙️
- Guitar 🎸
- Mouth 👄

### Level editor

Before implementing any mechanics we need some ground to jump to/from. So the first step was creating a simple level editor.

![early editor](images/early-editor.gif)

The level itself is just segments as collision surfaces and triangles as background tiles.

By adding snapping to already existing vertices we make sure level is properly connected.

![editor](images/editor.gif)

*As a postjam mechanics I also added wind to the game which is just another background type but thats moving, so an option to make background moving in desired direction was also added to the editor*.

### Physics

Now that we have the ground it is time to add player and implement the gameplay.

![early gameplay](images/early-gameplay.gif)

As most of the things, physics for this game was written from scratch. It's not too hard of a task since the player is just a circle rolling around, so we just need to:

- Find the collision with the level using simple geometry. In case we intersect with multiple surfaces we want to select the one with deepest penetration. Why? I don't remember 🤣.

  ```rust
  for surface in &level.surfaces {
      let v = surface.vector_from(guy.pos);
      let penetration = self.config.guy_radius - v.len();
      if penetration > EPS && Vec2::dot(v, guy.vel) > 0.0 {
          let collision = Collision {
              penetration,
              normal: -v.normalize_or_zero(),
              surface_params: &surface.params,
          };
          collision_to_resolve =
              std::cmp::max_by_key(collision_to_resolve, Some(collision), |collision| {
                  collision.map(|c| c.penetration)
              });
      }
  }
  ```

- Resolve that collision using simple physics known to every student (I am pretty sure I see a small bug 🐛 in this code but whatever):

  ```rust
  if let Some(collision) = collision_to_resolve {
      guy.pos += collision.normal * collision.penetration;
      let normal_vel = Vec2::dot(guy.vel, collision.normal);
      let tangent = collision.normal.rotate_90();
      let tangent_vel = Vec2::dot(guy.vel, tangent) - guy.w * guy.radius;
      guy.vel -=
          collision.normal * normal_vel * (1.0 + collision.surface_params.bounciness);
      let max_friction_impulse = normal_vel.abs() * collision.surface_params.friction;
      let friction_impulse = -tangent_vel.clamp_abs(max_friction_impulse);
      guy.vel += tangent * friction_impulse;
      guy.w -= friction_impulse / guy.radius;
  }
  ```

I use Arch btw 🦀

![gameplay](images/gameplay.gif)

### Graphics

For the graphics I used the same style I did previously (for [Extremely Extreme Sports](kuviman.itch.io/extremely-extreme-sports)). I am still unsure how to call this style. Its not exactly pixel art, but a silly quick ~~MS~~-Paint stuff.

![ees character](images/ees-character.png)

In fact, I even started [my own paint program](https://github.com/kuviman/yeti-draw) with just what I need - drawing pixels, infinite canvas, online collaboration. Although I never finished it, so it was not used this time 😞. Some day...

![yeti-draw](images/yeti-draw.png)

One of the things I think I improved on is that I did not add outlines since we don't need to have a lot of details in this style.

I think the character turned out to be really nice, along with the animation of shaking cheeks and eyes going red I was happy enough with it. In fact, I was too afraid to make it worse which is why I didn't add more character customization options like last time, so you can only randomize the colors of skin/hair/clothes.

![character](images/character.gif)

### Audio

Not many sound effects were made for this game, but only the most important one - farting.

So I put the microphone in front of me and just do it. Yes, with my mouth. Several times so that we can randomize the sound effect each time. About 10 seconds and voila, the sfx is ready.

![recording farts](images/recording-farts.gif)

Now to fill the remaining silence there has to be some music. I am not very good with it, but... Grabbed the guitar, played 10 seconds of 2 notes for bass and then 10 seconds of just some random notes and that was it. 20 seconds in total to create the music track. Amazing! Turned out to be much better I thought it would be.

Later the game music was covered by Brainoid, so if you want to hear a much better version, [here's the video - Brainoid also did covers for multiple other jam games](https://www.youtube.com/watch?v=J_gNntsQnWQ).

I do intend to try get better at making music, so I want to make a cover of Brainoid's cover of my thing. Tried once, failed miserably. I blame this for my failure 😅, no idea how that happened:

![broken guitar](images/broken-guitar.jpg)

Some day...

### Multiplayer

Multiplayer is hard. Thats what people say. That is likely true.
In case you want realtime multiplayer with physics interaction between the players. That requires prediction, interpolation and sounds scary.

So, to make this easy I just make singleplayer game where you can see other people playing at the same time.

![early multiplayer](images/early-multiplayer.gif)

Last time I did use interpolation for other players, which also lead to small issue - you could see people going through obstacles. So to both fix this issue and make my life even easier this time I just basically played a recording of other people, simulating them in between keyframes (when input is changed) exactly the same way as your own player. Yes, it does mean that you see other people slightly behind in time, but that is good enough of an experience and really easy to implement too.

That said, you can still "interact" with other people by using emotes, laughing at them as they fall or raging when you fall yourself.

![emotes](images/emotes.gif)

### Level design

This was the first time I actually had to do that work.
Usually I just do some simple random generation and thats it.

But for this game to work it is important to have a tutorial area where just single farts can work, going into jumps requiring double farting, introducing new mechanics and harder jumps later, figuring out where are the speedrun skips and the most enraging falls.

![routes](images/routes.png)

Overall I'd say the level ended up ok, except for two problematic areas. The trampoline I put on top of the tutorial area allowed for skipping half of the game (although it is not very easy) and some people who played never realized that it was not the intended route - it was supposed to be used as a safe mechanism to recover after fall. And even if you go the intended route it is hard to see where you need to go - you are supposed to go slightly down at some point but thats not what people look for.

So to fix both of those I have introduced wind mechanics in the postjam version of the game. Both to visualize the intended route and to nerf the trampoline skip (but still allow recovering from falls).

![wind](images/wind.gif)

### Finishing the work

As the end of the development time was approaching, there was one thing I still have left to do - finishing the game myself 🤣.

I did test all the jumps before separately, but going from start to finish requires some time. So this had to be the last step.

As far as I remember, I did it in about 40 minutes.

![tokei](images/tokei.png)

![git log](images/gitlog.png)

As always in game jams, code is not very good but it works 🍝.

Total time spend on making the game: 20 hours.

Here's a timelapse video of the development:

<https://www.youtube.com/watch?v=zxApycDzn78>

### Postjam updates

There have been some updates I did after the time was up.
As people need to rate the game based on the initial version those were pretty secret, activated by typing `postjamplease` as your name.

The list of things done:

- Wind mechanics as mentioned before to nerf trampoline and help players with seeing intended route 🍃.
- Bouncy cave (one of the hardest parts of the level) was made a bit easier
- Leaderboard 🥇
- Secret spectator mode for the tournament (more on that later)
- A bit of water to show off more possibilities for potential future level updates 🤽.
- Very secret unicorn cave next to spawn 🦄

![unicorn](images/unicorn.gif)

### Play+Rate

After the game was done is what real fun begins. People play your game. You play other people games.

And it is even more fun if done on stream. I have submitted my game to a bunch of streamers, and you can see a compilation of them here:

<https://www.youtube.com/watch?v=dd9-6KY7-6k>

Only 2 of the highlighted people managed to finish the game before ragequitting: it took [Pomo](https://twitch.tv/pomothedog) 50 minutes and [itsboats](https://twitch.tv/itsboats) 30 minutes.

[HonestDanGames](https://www.twitch.tv/honestdangames) was playing the game for more than 2 hours before finally giving up:

![HonestDanGames](images/honestdan.png)

But the reward for the most dedication goes to rickylee who managed to finish the game after more than 3 hours 🤯.

Overall, I know only of 10 people other than myself that did finish, as you can see on the [LD game page](https://ldjam.com/events/ludum-dare/51/getting-farted-on).

As you can see it doesn't take long to beat the game for the speedrunners. In fact, Meep's speedrun in **1 minute 50 seconds** was done shortly after publishing the game on the first day!

Another interesting thing is this Nertsal's tool assisted speedrun of jam version in **53 seconds**:

<https://www.youtube.com/watch?v=AgUilKRflBU>

I myself also played a lot of other games on my stream (MORE THAN 100!!!). Since there is a lot of active people submitting games I used my [Raffle Royale](https://github.com/kuviman/ttv) thing I showed previously for selecting a random person to play their game next.
This turned out to be a pretty fun way to do it, so I will work more on that. Some day...

![Raffle Royale](images/raffle-royale.gif)

Here's a list of some of my favourites games from LD51:

- [Renew my Subscription](https://ldjam.com/events/ludum-dare/51/renew-my-subscription)
- [Last Pawn Standing - Chess Battle Royale](https://ldjam.com/events/ludum-dare/51/last-pawn-standing-chess-battle-royale) - requires several people to play (online multiplayer)
- [Blink](https://ldjam.com/events/ludum-dare/51/blink)
- [Dr. Word's Machine](https://ldjam.com/events/ludum-dare/51/dr-words-machine)
- [Love is Legal: Speed Dating For Criminals](https://ldjam.com/events/ludum-dare/51/love-is-legal-speed-dating-for-criminals)
- [A Handy Job](https://ldjam.com/events/ludum-dare/51/a-handy-job)
- [Intern Inferno](https://ldjam.com/events/ludum-dare/51/intern-inferno) - *found this in danger zone*
- [Curse of Tencond](https://ldjam.com/events/ludum-dare/51/the-curse-of-tencond)
- [10 Second Mixtape](https://ldjam.com/events/ludum-dare/51/10-second-mixtape)
- [Gelato Drift](https://ldjam.com/events/ludum-dare/51/gelato-drift)

And I could continue this list for much longer, but I have to stop somewhere 🤣.

### Score Chasers Tournament

Playing the games is fun, but some people (me included) have even more fun when competing.

So this time, same as last LD there was a Score Chasers Tournament where 6 games were played for highscores, including my game. And, same as last time, I made custom skins for all the participants.

![custom skins](images/custom-skins.png)

Turns out, not even every score chaser can beat the game I made. Out of 8 participants only 6 finished, with time ranging from 2 to 20 minutes.

Now while obviously I took 1st place in my own game (since Meep and Bogden, the winners of last tournament, could not participate this time), I did lose some points in other games due to RNG nature of most of them, so I ended up taking 2nd place overall on the tournament leaderboard.

Here's the video of the tournament (my game was played last):

{{ youtube(id="SKv0YoJmp68") }}

# Results

It is expected that this game, as being insanely hard would not get very high ratings like [I did last time](https://kuviman.itch.io/extremely-extreme-sports/devlog/372532/extremely-extreme-sports-postmortem),
but I still did pretty good here overall.

And, apparently farting is really good humor after all.

![results](images/results.png)

# The end

Thanks for reading and enjoy having *The Story of Getting Farted On* in your browser history <3

🔗 Here links for my things:

- [GitHub](https://github.com/kuviman)
- [Discord](https://discord.gg/qPuvJ3fT7u)
- [Twitch](https://twitch.tv/kuviman)
- [YouTube](https://www.youtube.com/channel/UCQCiBwBzzuGbSjCoM9CslKw)
