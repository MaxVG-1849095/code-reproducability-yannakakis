select count(*) from imdb100, imdb118, imdb39 where imdb100.d = imdb118.d and imdb118.d = imdb39.s;