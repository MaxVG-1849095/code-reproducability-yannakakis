select count(*) from imdb100, imdb124, imdb51 where imdb100.d = imdb124.d and imdb124.d = imdb51.s;