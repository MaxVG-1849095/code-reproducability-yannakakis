select count(*) from imdb100, imdb124, imdb60 where imdb100.d = imdb124.d and imdb124.d = imdb60.s;