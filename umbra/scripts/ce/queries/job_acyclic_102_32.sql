select count(*) from imdb100, imdb127, imdb63 where imdb100.d = imdb127.d and imdb127.d = imdb63.s;