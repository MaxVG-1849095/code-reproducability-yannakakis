select count(*) from imdb100, imdb121, imdb44, imdb8 where imdb100.d = imdb121.d and imdb121.d = imdb44.s and imdb44.s = imdb8.s;