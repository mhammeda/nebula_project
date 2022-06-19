<template>
    <div class="individualPost py-2">
        <v-card elevation="4" max-width="1000" shaped color="secondary">
            <v-app-bar elevation="0" color="secondary" class="py-0">
                <v-btn
                    small
                    plain
                    text
                    color="primary"
                    @click="redirrectCommunity()"
                    ><b>{{ postInfo.community }}</b></v-btn
                >
                <div class="overline accent--text">
                    {{ getCreatedDate() }}
                </div>
                <v-spacer> </v-spacer>

                <v-btn icon @click="openFullPost">
                    <v-icon color="accent">
                        mdi-open-in-new
                    </v-icon>
                </v-btn>
            </v-app-bar>

            <v-container>
                <v-row no-gutters>
                    <!-- post area -->
                    <v-col cols="16">
                        <v-list color="secondary" three-line class="py-0">
                            <v-list-item>
                                <v-tooltip bottom>
                                    <template v-slot:activator="{ on, attrs }">
                                        <button @click="redirrectUserPage()">
                                            <v-list-item-avatar
                                                size="75"
                                                color="primary"
                                                v-bind="attrs"
                                                v-on="on"
                                            >
                                                <v-img
                                                    :src="userProfilePicLink"
                                                />
                                            </v-list-item-avatar>
                                        </button>
                                    </template>
                                    <span>{{ postInfo.author.username }}</span>
                                </v-tooltip>

                                <!-- regular post -->
                                <v-list-item-content
                                    v-if="!editingPost"
                                    class="text-left white--text"
                                >
                                    <v-list-item-title>{{
                                        postInfo.title
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle
                                        v-if="postInfo.content[0].text"
                                        class="white--text"
                                        >{{
                                            postInfo.content[0].text.text
                                        }}</v-list-item-subtitle
                                    >
                                    <v-list-item-subtitle
                                        v-else-if="postInfo.content[0].markdown"
                                        class="white--text"
                                        >{{
                                            postInfo.content[0].markdown.text
                                        }}</v-list-item-subtitle
                                    >
                                </v-list-item-content>

                                <!-- editing post -->
                                <v-list-item-content
                                    v-else
                                    class="text-left white--text"
                                >
                                    <v-list-item
                                        ><v-text-field
                                            solo
                                            outlined
                                            v-model="editedTitle"
                                        ></v-text-field
                                    ></v-list-item>
                                    <v-list-item
                                        ><v-textarea
                                            solo
                                            outlined
                                            auto-grow
                                            dense
                                            rows="2"
                                            v-model="editedContent"
                                        ></v-textarea
                                    ></v-list-item>
                                </v-list-item-content>
                            </v-list-item>
                        </v-list>
                    </v-col>
                    <!-- media area -->
                    <v-col cols="2">
                        <v-img
                            v-if="checkForImage()"
                            class="mx-8 d-flex align-end flex-column"
                            :src="imageUrl"
                            height="100"
                            width="100"
                            contain
                        />
                    </v-col>
                </v-row>
            </v-container>

            <div v-if="editingPost" justify="center" align="center">
                <v-btn
                    color="green"
                    class="mx-3"
                    @click="sendPostEdit"
                    :loading="editedLoadingIcon"
                    >save</v-btn
                >
                <v-btn color="white" @click="editingPost = false">cancel</v-btn>
            </div>

            <v-card-actions>
                <v-list-item>
                    <v-btn icon
                        ><v-icon color="accent">mdi-thumb-up</v-icon></v-btn
                    >

                    <v-btn icon
                        ><v-icon color="accent">mdi-thumb-down</v-icon></v-btn
                    >

                    <v-row justify="end">
                        <div v-if="checkPostBelongsToUser()">
                            <v-btn
                                icon
                                @click="
                                    confirmDeleteDialog = !confirmDeleteDialog
                                "
                            >
                                <v-icon color="red">mdi-delete</v-icon>
                            </v-btn>
                            <v-btn icon @click="editingPost = !editingPost">
                                <v-icon color="accent">mdi-pencil</v-icon>
                            </v-btn>
                        </div>
                        <v-btn icon @click="overlay = !overlay">
                            <v-icon color="accent">
                                mdi-reply
                            </v-icon>
                        </v-btn>
                        <v-btn
                            icon
                            @click="getComments()"
                            :loading="loading"
                            :disabled="loading"
                        >
                            <v-icon color="accent">
                                mdi-comment-multiple
                            </v-icon>
                            <template v-slot:loader>
                                <span class="custom-loader white--text">
                                    ...
                                </span>
                            </template>
                        </v-btn>
                    </v-row>
                </v-list-item>
            </v-card-actions>
            <div v-show="show">
                <div v-if="postInfo.children.length > 0">
                    <Comment
                        v-for="comment in shownComments"
                        :key="comment.id"
                        :comment="comment"
                        v-on:postSuccesful="refreshReplies"
                        v-on:deleteComment="refreshReplies"
                    ></Comment>
                </div>
                <div
                    v-else
                    justify="center"
                    align="center"
                    class="white--text body-1"
                >
                    no comments yet
                </div>
                <div v-if="numbCommentsShown < postInfo.children.length">
                    <v-btn text @click="showMoreComments()" class="accent--text"
                        >show more</v-btn
                    >
                </div>
            </div>
        </v-card>
        <v-overlay :z-index="zIndex" :value="overlay">
            <ReplyBox
                v-on:closeBox="overlay = false"
                v-on:postSuccesful="refreshReplies"
                :item="item"
            />
        </v-overlay>
        <v-dialog v-model="confirmDeleteDialog" max-width="450">
            <v-card>
                <v-card-title
                    >Are you sure you want to delete this post?</v-card-title
                >
                <v-card-actions>
                    <v-btn text plain @click="confirmDeleteDialog = false">
                        cancel
                    </v-btn>
                    <v-btn
                        text
                        color="red"
                        :loading="deleteLoading"
                        plain
                        @click="deletePost"
                    >
                        delete
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </div>
</template>

<script>
import Comment from './comment.vue'
import ReplyBox from './ReplyBox.vue'
export default {
    props: ['item'],
    mounted() {
        // get the user profile pic
        // get user info first
        const url = `/internal/users/${this.item.author.username}`

        this.$http
            .get(url, {}, { withCredentials: true })
            .then(response => {
                if (response.data.length !== 0) {
                    const userInfo = response.data
                    if (userInfo.avatarUrl) {
                        this.userProfilePicLink = userInfo.avatarUrl
                    }
                } else {
                    this.errorMessage = 'Error getting user data'
                    // this.errorSnackbar = true
                }
            })
            .catch(error => {
                this.errorMessage = 'Error getting user data'
                // this.errorSnackbar = true
            })
    },

    data() {
        return {
            url: '/internal/posts',
            // overlay variables
            show: false,
            overlay: false,
            zIndex: 10,
            comments: [],
            postInfo: this.item,
            loading: false,
            commentButtonText: 'See Comments',
            confirmDeleteDialog: false,
            deleteLoading: false,

            // comment list
            numbCommentsShown: 10, // max 10 by default
            shownComments: [],

            // images
            userProfilePicLink: require('../../assets/defaultUserIcon.jpg'),
            imageUrl: '',

            // error handling
            errorSnackbar: false,
            errorMessage: '',

            // editing post
            editingPost: false,
            editedTitle: this.item.title,
            // needs updating
            editedContent: this.item.content[0].text
                ? this.item.content[0].text.text
                : this.item.content[0].markdown.text,
            editedLoadingIcon: false,
        }
    },
    components: {
        Comment,
        ReplyBox,
    },
    methods: {
        getComments() {
            this.errorMessage = ''

            if (!this.show) {
                this.loading = true
                const reqUrl = `${this.url}/${this.item.id}`
                this.$http
                    .get(reqUrl, {}, { withCredentials: true })
                    .then(response => {
                        this.postInfo = response.data
                        const comments = this.postInfo.children
                        if (comments.length > this.numbCommentsShown) {
                            this.shownComments = comments.slice(
                                0,
                                this.numbCommentsShown
                            )
                        } else {
                            this.shownComments = comments.slice()
                        }

                        this.loading = false
                        this.commentButtonText = 'Hide Comments'
                    })
                    .catch(error => {
                        console.log(error.message)
                        this.loading = false
                        this.show = false
                        this.errorMessage = 'error retrieving comments'
                        this.errorSnackbar = true
                    })
            } else {
                this.commentButtonText = 'See Comments'
            }

            this.show = !this.show
        },

        refreshReplies() {
            this.overlay = false
            if (this.show) {
                this.show = !this.show
                this.getComments()
            }
        },

        checkPostBelongsToUser() {
            return (
                this.item.author['username'] === localStorage.getItem('userid')
            )
        },
        deletePost(e) {
            e.preventDefault()
            this.deleteLoading = true
            const url = `${this.url}/${this.item.id}`
            console.log(url)
            this.$http
                .delete(url, {}, { withCredentials: true })
                .then(response => {
                    this.$emit('deletePost', this.item)
                    this.deleteLoading = false
                    this.confirmDeleteDialog = false
                })
                .catch(error => {
                    console.error(error)
                    this.deleteLoading = false
                    this.confirmDeleteDialog = false
                    this.errorSnackbar = true
                    this.errorMessage = 'error deleting post'
                })
        },

        openFullPost() {
            const url = `/post/${this.item.id}`
            //https://stackoverflow.com/questions/4907843/open-a-url-in-a-new-tab-and-not-a-new-window
            const win = window.open(url, '_blank')
            win.focus()
        },

        redirrectCommunity() {
            const url = `community/${this.item.community}`
            this.$router.push(url)
        },

        redirrectUserPage() {
            const url = `user/${this.item.author.username}`
            this.$router.push(url)
        },

        getCreatedDate() {
            const d = new Date(0)
            d.setUTCSeconds(this.item.created)
            return d.toUTCString()
        },

        sendPostEdit() {
            const url = `${this.url}/${this.item.id}`
            this.editedLoadingIcon = true

            this.$http
                .put(
                    url,
                    {
                        title: this.editedTitle,
                        content: [
                            {
                                markdown: {
                                    text: this.editedContent,
                                },
                            },
                        ],
                    },
                    { withCredentials: true }
                )
                .then(response => {
                    this.postInfo = response.data
                    this.editingPost = false
                    this.editedLoadingIcon = false
                })
                .catch(error => {
                    this.errorSnackbar = true
                    this.errorMessage = 'error editing post'
                    this.editedLoadingIcon = false
                })
        },

        checkForImage() {
            const content = this.postInfo.content
            // not the sexiest code coming up
            // iterate through content and look for "<img "
            for (let i = 0; i < content.length; i++) {
                const current = content[0]
                // for now images only in markdown currently
                if (current.markdown) {
                    const markdownText = current.markdown.text
                    const matches = markdownText.match(
                        /<img src="[A-z/\-0-9]+" width="300" \/>/g
                    )
                    // found a match
                    if (matches) {
                        // just take first image
                        const result = matches[0]
                        // grab the url
                        const urlString = result.match(/src="[A-z/\-0-9]+"/g)[0]
                        if (urlString) {
                            let url = urlString.match(/"[A-z/\-0-9]+"/g)[0]
                            if (url) {
                                url = url.substring(1, url.length - 1)
                                this.imageUrl = url
                                return true
                            }
                        }
                    }
                }
            }

            return false
        },

        showMoreComments() {
            if (this.numbCommentsShown < this.postInfo.children.length) {
                // add on remaining comments or 10 more, whichever is smaller
                if (
                    this.postInfo.children.length >
                    this.numbCommentsShown + 10
                ) {
                    this.numbCommentsShown = this.numbCommentsShown + 10
                    this.shownComments = this.postInfo.children.slice(
                        0,
                        this.numbCommentsShown
                    )
                } else {
                    this.numbCommentsShown = this.postInfo.children.length
                    this.shownComments = this.postInfo.children.slice()
                }
            }
        },
    },
}
</script>

<style scoped>
.individualPost {
    margin-right: 20px;
}
.v-btn::before {
    background-color: transparent;
}

.v-btn i:hover {
    transform: scale(1.15);
}
</style>
